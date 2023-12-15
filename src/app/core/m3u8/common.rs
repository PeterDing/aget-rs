use std::collections::HashMap;

use m3u8_rs::{parse_playlist_res, Key, Playlist};

use crate::common::{
    bytes::bytes::{decode_hex, u32_to_u8x4},
    errors::{Error, Result},
    list::SharedVec,
    net::{
        net::{join_url, redirect, request},
        HttpClient, Method, Url,
    },
};

#[derive(Debug, Clone)]
pub struct M3u8Segment {
    pub index: u64,
    pub method: Method,
    pub url: Url,
    pub data: Option<String>,
    pub key: Option<[u8; 16]>,
    pub iv: Option<[u8; 16]>,
}

pub type M3u8SegmentList = Vec<M3u8Segment>;

pub type SharedM3u8SegmentList = SharedVec<M3u8Segment>;

pub async fn get_m3u8(
    client: &HttpClient,
    method: Method,
    url: Url,
    data: Option<String>,
) -> Result<M3u8SegmentList> {
    // url -> (key, iv)
    let mut keymap: HashMap<Url, [u8; 16]> = HashMap::new();
    let mut urls = vec![url];
    let mut list = vec![];

    let mut idx = 0;

    while let Some(url) = urls.pop() {
        tracing::debug!("m3u8 url: {}", url);
        let u = redirect(client, method.clone(), url.clone(), data.clone()).await?;

        if u != url {
            tracing::debug!("m3u8 redirect to: {}", u);
            urls.push(u.clone());
            continue;
        }

        let base_url = u.clone();

        // Read m3u8 content
        let resp = request(client, method.clone(), u.clone(), data.clone(), None).await?;
        let cn = resp.bytes().await?;
        let mut cn = cn.to_vec();

        // Adding "\n" for the case when response content has not "\n" at end.
        cn.extend(b"\n");

        // Parse m3u8 content
        let parsed = parse_playlist_res(cn.as_ref());
        match parsed {
            Ok(Playlist::MasterPlaylist(mut pl)) => {
                pl.variants.reverse();
                for variant in &pl.variants {
                    let url = join_url(&base_url, &variant.uri)?;
                    urls.push(url);
                }
            }
            Ok(Playlist::MediaPlaylist(pl)) => {
                let mut index = pl.media_sequence as u64;
                let mut key_m: Option<Key> = None;
                for segment in &pl.segments {
                    let seg_url = join_url(&base_url, &segment.uri)?;

                    // In `pl.segment`, the same key will not repeat, if previous key appears.
                    let segment_key = if segment.key.is_none() && key_m.is_some() {
                        &key_m
                    } else {
                        key_m = segment.key.clone();
                        &segment.key
                    };

                    let (key, iv) = if let Some(key) = segment_key {
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
                        if let Some(url) = &key.uri {
                            let key_url = join_url(&base_url, url)?;
                            if let Some(k) = keymap.get(&key_url) {
                                (Some(*k), Some(iv))
                            } else {
                                let k = get_key(client, Method::GET, key_url.clone()).await?;
                                keymap.insert(key_url.clone(), k);
                                tracing::debug!(
                                    "Get key: {}, iv: {}",
                                    unsafe { std::str::from_utf8_unchecked(&k) },
                                    unsafe { std::str::from_utf8_unchecked(&iv) }
                                );
                                (Some(k), Some(iv))
                            }
                        } else {
                            (None, None)
                        }
                    } else {
                        (None, None)
                    };

                    list.push(M3u8Segment {
                        index: idx,
                        method: Method::GET,
                        url: seg_url.clone(),
                        data: None,
                        key,
                        iv,
                    });
                    index += 1;
                    idx += 1;
                }
            }
            Err(_) => return Err(Error::M3U8ParseFail),
        }
    }
    Ok(list)
}

async fn get_key(client: &HttpClient, method: Method, url: Url) -> Result<[u8; 16]> {
    let resp = request(client, method.clone(), url.clone(), None, None).await?;
    let cn = resp.bytes().await?;
    let mut buf = [0; 16];
    buf[..].clone_from_slice(&cn);
    Ok(buf)
}
