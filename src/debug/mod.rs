pub mod render;

#[derive(Clone, Debug)]
pub struct ComputeDebugInfo {
    pub mean_cps: u32,
}

#[derive(Clone, Debug)]
pub struct DebugInfo<'a> {
    pub compute: &'a ComputeDebugInfo,
    pub mean_fps: u32
}

impl ComputeDebugInfo {
    pub fn initial() -> ComputeDebugInfo {
        ComputeDebugInfo { mean_cps: 0 }
    }

    pub fn add_render_info(&self, mean_fps: u32) -> DebugInfo {
        DebugInfo {
            mean_fps,
            compute: self,
        }
    }
}
