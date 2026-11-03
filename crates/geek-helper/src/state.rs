//! What must survive the helper: whether a kill switch is engaged (and
//! strict), and which sing-box it left running if it died.

use std::path::{Path, PathBuf};

use geek_ipc::KillSwitch;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct State {
    pub kill_switch: Option<KillSwitch>,
    pub engine_pid: Option<u32>,
}

pub struct StateFile(PathBuf);

impl StateFile {
    pub fn new(dir: &Path) -> Self {
        Self(dir.join("state.json"))
    }

    pub fn load(&self) -> State {
        std::fs::read(&self.0).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    /// Atomic (temp file, fsync, then rename): a half-written state after a
    /// power cut would lose a strict kill switch.
    pub fn save(&self, s: &State) {
        let tmp = self.0.with_extension("json.tmp");
        if let Ok(bytes) = serde_json::to_vec(s) {
            if let Ok(mut f) = std::fs::File::create(&tmp) {
                use std::io::Write;
                if f.write_all(&bytes).is_ok() && f.sync_all().is_ok() {
                    let _ = std::fs::rename(&tmp, &self.0);
                }
            }
        }
    }
}
