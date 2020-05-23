use std::collections::HashMap;

use m3u8_rs::{parse_playlist_res, playlist::Playlist};

use crate::common::{
    bytes::{
        bytes::{decode_hex, u32_to_u8x4},
        bytes_type::Bytes,
    },
    errors::{Error, Result},
    list::SharedVec,
    net::{
        net::{redirect, request},
        net_type::{HttpClient, Method, ResponseExt, Uri, Url},
    },
};

#[derive(Debug, Clone)]
pub struct M3u8Segment {
    pub index: u64,
    pub method: Method,
    pub uri: Uri,
    pub data: Option<Bytes>,
    pub key: Option<[u8; 16]>,
    pub iv: Option<[u8; 16]>,
}

impl M3u8Segment {
    pub fn new(
        index: u64,
        method: Method,
        uri: Uri,
        data: Option<Bytes>,
        key: Option<[u8; 16]>,
        iv: Option<[u8; 16]>,
    ) -> M3u8Segment {
        M3u8Segment {
            index,
            method,
            uri,
            data,
            key,
            iv,
        }
    }
}

pub type M3u8SegmentList = Vec<M3u8Segment>;

pub type SharedM3u8SegmentList = SharedVec<M3u8Segment>;

pub async fn get_m3u8(
    client: &HttpClient,
    method: Method,
    uri: Uri,
    data: Option<Bytes>,
) -> Result<M3u8SegmentList> {
    // uri -> (key, iv)
    let mut keymap: HashMap<Uri, ([u8; 16], [u8; 16])> = HashMap::new();
    let mut uris = vec![uri];
    let mut list = vec![];

    let mut idx = 0;

    while let Some(uri) = uris.pop() {
        debug!("m3u8", uri);
        let u = redirect(client, method.clone(), uri.clone(), data.clone()).await?;
        debug!("m3u8 redirect to", u);

        if u != uri {
            uris.push(u.clone());
            continue;
        }

        let base_url = Url::parse(&format!("{:?}", u))?;

        // Read m3u8 content
        let mut resp = request(client, method.clone(), u.clone(), data.clone(), None).await?;

        // Adding "\n" for the case when response content has not "\n" at end.
        let cn = resp.text()? + "\n";

        // Parse m3u8 content
        let parsed = parse_playlist_res(cn.as_bytes());
        match parsed {
            Ok(Playlist::MasterPlaylist(mut pl)) => {
                pl.variants.reverse();
                for variant in &pl.variants {
                    let url = base_url.join(&variant.uri)?;
                    let uri: Uri = url.as_str().parse()?;
                    uris.push(uri);
                }
            }
            Ok(Playlist::MediaPlaylist(pl)) => {
                let mut index = pl.media_sequence as u64;
                for segment in &pl.segments {
                    let seg_url = base_url.join(&segment.uri)?;
                    let seg_uri: Uri = seg_url.as_str().parse()?;

                    let (key, iv) = if let Some(key) = &segment.key {
                        let iv = if let Some(iv) = &key.iv {
                            let mut i = [0; 16];
                            let buf = decode_hex(&iv[2..])?;
                            i.clone_from_slice(&buf[..]);
                            i
                        } else {
                            let mut iv = [0; 16];
                            let index_bin = u32_to_u8x4(index as u32);
                            iv[12..].clone_from_slice(&index_bin);
                            iv
                        };
                        if let Some(uri) = &key.uri {
                            let key_url = base_url.join(&uri)?;
                            let key_uri: Uri = key_url.as_str().parse()?;
                            if let Some((k, iv)) = keymap.get(&key_uri) {
                                (Some(*k), Some(*iv))
                            } else {
                                let k = get_key(client, Method::GET, key_uri.clone()).await?;
                                keymap.insert(key_uri.clone(), (k, iv));
                                debug!("Get key, iv", (k, iv));
                                (Some(k), Some(iv))
                            }
                        } else {
                            (None, None)
                        }
                    } else {
                        (None, None)
                    };

                    list.push(M3u8Segment::new(
                        idx,
                        Method::GET,
                        seg_uri.clone(),
                        None,
                        key,
                        iv,
                    ));
                    index += 1;
                    idx += 1;
                }
            }
            Err(_) => return Err(Error::M3U8ParseFail),
        }
    }
    Ok(list)
}

async fn get_key(client: &HttpClient, method: Method, uri: Uri) -> Result<[u8; 16]> {
    let mut resp = request(client, method.clone(), uri.clone(), None, None).await?;
    let mut cn = [0; 16];
    resp.copy_to(&mut cn[..])?;
    Ok(cn)
}
