use std::{path::PathBuf, sync::Arc, time::Duration};

use librqbit::{
    api::TorrentIdOrHash, dht::PersistentDhtConfig, AddTorrent, AddTorrentOptions, AddTorrentResponse, Api,
    PeerConnectionOptions, Session, SessionOptions, SessionPersistenceConfig,
};
use url::Url;

use crate::{
    app::show::bt_show::BtShower,
    common::errors::{Error, Result},
    features::{args::Args, running::Runnable},
};

pub struct BtHandler {
    torrent_or_magnet: Url,
    output: PathBuf,
    file_regex: Option<String>,
    seed: bool,
    trackers: Option<Vec<String>>,
    peer_connect_timeout: Option<u64>,
    peer_read_write_timeout: Option<u64>,
    peer_keep_alive_interval: Option<u64>,
}

impl BtHandler {
    pub fn new(args: &(impl Args + std::fmt::Debug)) -> BtHandler {
        tracing::debug!("BtHandler::new");

        BtHandler {
            torrent_or_magnet: args.url(),
            output: args.output(),
            file_regex: args.bt_file_regex(),
            seed: args.seed(),
            trackers: args.bt_trackers(),
            peer_connect_timeout: args.bt_peer_connect_timeout(),
            peer_read_write_timeout: args.bt_peer_read_write_timeout(),
            peer_keep_alive_interval: args.bt_peer_keep_alive_interval(),
        }
    }

    async fn start(self) -> Result<()> {
        tracing::debug!("BtHandler::start");

        let output_dir = &self.output;
        let persistence_dir = output_dir
            .join("..")
            .join(output_dir.file_name().unwrap().to_string_lossy().to_string() + ".bt.aget");
        let dht_config_filename = persistence_dir.join("dht.json");

        // 0. Check whether task is completed
        tracing::debug!("BtHandler: check whether task is completed");
        if output_dir.exists() && !persistence_dir.exists() {
            return Ok(());
        }

        // 1. Create session
        tracing::debug!("BtHandler: create session");
        let sopts = SessionOptions {
            disable_dht: false,
            disable_dht_persistence: false,
            dht_config: Some(PersistentDhtConfig {
                config_filename: Some(dht_config_filename),
                ..Default::default()
            }),
            peer_id: None,
            peer_opts: Some(PeerConnectionOptions {
                connect_timeout: self.peer_connect_timeout.map(Duration::from_secs),
                read_write_timeout: self.peer_read_write_timeout.map(Duration::from_secs),
                keep_alive_interval: self.peer_keep_alive_interval.map(Duration::from_secs),
            }),
            fastresume: true,
            persistence: Some(SessionPersistenceConfig::Json {
                folder: Some(persistence_dir.clone()),
            }),
            ..Default::default()
        };
        let session = Session::new_with_opts(output_dir.to_owned(), sopts)
            .await
            .map_err(|err| Error::BitTorrentError(err.to_string()))?;

        // 2. Create shower
        tracing::debug!("BtHandler: create shower");
        let stats_watcher = StatsWatcher {
            session: session.clone(),
            forever: self.seed,
        };
        let stats_watcher_join_handler = actix_rt::spawn(stats_watcher.watch());

        // 3. Add torrent or magnet
        tracing::debug!("BtHandler: add torrent or magnet");
        let topts = Some(AddTorrentOptions {
            only_files_regex: self.file_regex.clone(),
            trackers: self.trackers.clone(),
            ..Default::default()
        });
        let response = session
            .add_torrent(
                AddTorrent::from_cli_argument(&self.torrent_or_magnet.as_str())
                    .map_err(|err| Error::BitTorrentError(err.to_string()))?,
                topts,
            )
            .await
            .map_err(|err| Error::BitTorrentError(err.to_string()))?;

        match response {
            AddTorrentResponse::AlreadyManaged(id, handle) => {
                tracing::debug!("Torrent {} is already managed", id);
                handle
                    .wait_until_completed()
                    .await
                    .map_err(|err| Error::BitTorrentError(err.to_string()))?;
            }
            AddTorrentResponse::Added(id, handle) => {
                tracing::debug!("Torrent {} is added", id);
                handle
                    .wait_until_completed()
                    .await
                    .map_err(|err| Error::BitTorrentError(err.to_string()))?;
            }
            _ => {
                unreachable!()
            }
        }

        // 4. Start seeding
        if self.seed {
            tracing::debug!("BtHandler: start seeding");
            println!("\nSeeding...");
        }
        while self.seed {
            actix_rt::time::sleep(Duration::from_secs(1)).await;
        }

        // 5. Exit shower
        tracing::debug!("BtHandler: exit shower");
        stats_watcher_join_handler.await.unwrap();

        // 6. Remove persistence folder
        tracing::debug!("BtHandler: remove persistence folder");
        std::fs::remove_dir_all(persistence_dir)?;

        Ok(())
    }
}

impl Runnable for BtHandler {
    fn run(self) -> Result<()> {
        let sys = actix_rt::System::new();
        sys.block_on(self.start())
    }
}

struct StatsWatcher {
    session: Arc<Session>,
    forever: bool,
}

impl StatsWatcher {
    async fn watch(self) {
        let mut shower = BtShower::new();

        let tid = TorrentIdOrHash::Id(0);
        let api = Api::new(self.session.clone(), None);

        let torrent_details = loop {
            if let Ok(torrent_details) = api.api_torrent_details(tid) {
                break torrent_details;
            }

            actix_rt::time::sleep(Duration::from_secs(1)).await;
        };

        if self.forever {
            shower.print_msg("Seed the torrent. Press Ctrl+C to exit").unwrap();
        }

        shower
            .print_name(torrent_details.name.as_deref().unwrap_or("unknown"))
            .expect("failed to print name");

        let torrent_files: Vec<_> = torrent_details.files.unwrap_or_default();
        let files: Vec<_> = torrent_files
            .iter()
            .map(|file| (file.name.as_str(), file.length, file.included))
            .collect();

        shower.print_files(&files[..]).unwrap();

        let mut completed_idx: Vec<bool> = vec![false; files.len()];

        loop {
            let stats = api.api_stats_v1(tid).expect("failed to get stats");

            if let Some(live) = stats.live {
                let completed = stats.progress_bytes;
                let total = stats.total_bytes;
                let down_rate = live.download_speed.mbps * 1e6;
                let up_rate = live.upload_speed.mbps * 1e6;
                let uploaded = live.snapshot.uploaded_bytes;

                let eta = {
                    let remains = total - completed;
                    // rate > 1.0 for overflow
                    if remains > 0 && down_rate > 1.0 {
                        let eta = (remains as f64 / down_rate) as u64;
                        // eta is large than 99 days, return 0
                        if eta > 99 * 24 * 60 * 60 {
                            0
                        } else {
                            eta
                        }
                    } else {
                        0
                    }
                };

                let peer_stats = live.snapshot.peer_stats;
                let live_peers = peer_stats.live;
                let queued_peers = peer_stats.queued;

                shower
                    .print_status(
                        completed,
                        total,
                        eta,
                        down_rate,
                        up_rate,
                        uploaded,
                        live_peers,
                        queued_peers,
                    )
                    .unwrap();

                files.iter().enumerate().for_each(|(i, (filename, length, included))| {
                    if *included && !completed_idx[i] {
                        let completed_size = stats.file_progress[i];
                        if completed_size == *length {
                            shower.print_completed_file(filename).unwrap();
                            completed_idx[i] = true;
                        }
                    }
                })
            }

            if !self.forever && stats.finished {
                break;
            }

            actix_rt::time::sleep(Duration::from_secs(1)).await;
        }
    }
}
