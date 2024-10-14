use std::{path::PathBuf, process::Command};

use crate::common::errors::Result;

pub(crate) struct FFmpegExecutor;

impl FFmpegExecutor {
    fn fix_m3u8(file_path: &PathBuf) -> Result<()> {
        // ffmpeg -y -loglevel repeat+info -i file:{file_path} -map 0 -dn -ignore_unknown -c copy -f {ext} -bsf:a aac_adtstoasc -movflags +faststart file:{file_path}.temp.{ext}
        let mut cmd = Command::new("ffmpeg");

        cmd.arg("-y")
            .arg("-loglevel")
            .arg("repeat+info")
            .arg("-i")
            .arg(format!("file:{}", file_path.display()))
            .arg("-map")
            .arg("0")
            .arg("-dn")
            .arg("-ignore_unknown")
            .arg("-c")
            .arg("copy")
            .arg("-bsf:a")
            .arg("aac_adtstoasc")
            .arg("-movflags")
            .arg("+faststart")
            .arg(format!("file:{}.temp.mp4", file_path.display()));

        // if file_path.ends_with(".mp4") {
        //     cmd.arg("-f").arg("mp4")
        // }
        Ok(())
    }
}
