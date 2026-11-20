//! FORMATIONS - many objects filling a shape, arriving over time (C# namespace
//! `CodeBrix.Imaging.Drawing.Formations`): the [`FormationArea`] to fill, the target points
//! ([`FormationTargets::fill`]) ranked by an arrival order ([`IArrivalOrder`]: random, centre
//! out, directional, a heap that climbs from the ground, or a function of your own), the way each
//! member gets there ([`IArrivalPath`]: straight, curved, spiralling, a swaying fall), when it
//! arrives ([`ArrivalSchedule`] with an [`ArrivalRateProfile`]), and
//! [`DrawingAnimationsMut::bind_formation`](crate::models::DrawingAnimationsMut::bind_formation),
//! which turns members into time-keyed elements of a session. Every value is a function of the
//! time passed in.
//!
//! IDENTICAL ARITHMETIC (NoSkia's own notes, kept): the area queries are float +, -, *, / and
//! square roots in a fixed order on the flattened polylines; the mask distances are double +,
//! -, *, / and square roots. NoSkia documents these math-library calls as the exceptions to its
//! bit-for-bit promise between its two back ends: SwayingFallArrival (sin, cos, exp),
//! SpiralArrival and DirectionalArrivalOrder (cos, sin), ArrivalRateProfile (pow) and
//! HeapArrivalOrder's lumps (DrawingNoise). Here the same calls go to Rust's math library: on the
//! host that is the platform's C library (as .NET's are), in a browser (wasm32) Rust's own port
//! of it - so those values may differ from C#'s in the last bits there.

mod arrival_math;
mod arrival_rate_profile;
mod arrival_schedule;
mod center_out_arrival_order;
mod curved_arrival;
mod custom_arrival_order;
mod directional_arrival_order;
mod dotnet_sort;
mod formation_area;
mod formation_area_mask;
mod formation_elements;
mod formation_member;
mod formation_pose;
mod formation_target;
mod formation_target_options;
mod formation_targets;
mod heap_arrival_order;
mod i_arrival_order;
mod i_arrival_path;
mod mask_distance;
mod random_arrival_order;
mod spiral_arrival;
mod straight_arrival;
mod swaying_fall_arrival;

#[cfg(test)]
mod tests;

pub(crate) use formation_elements::driver as formation_elements_driver;
pub use arrival_rate_profile::ArrivalRateProfile;
pub use arrival_schedule::ArrivalSchedule;
pub use center_out_arrival_order::CenterOutArrivalOrder;
pub use curved_arrival::{CurvedArrival, CurvedArrivalOptions};
pub use custom_arrival_order::CustomArrivalOrder;
pub use directional_arrival_order::DirectionalArrivalOrder;
pub use formation_area::FormationArea;
pub use formation_elements::FormationElements;
pub use formation_member::FormationMember;
pub use formation_pose::FormationPose;
pub use formation_target::FormationTarget;
pub use formation_target_options::FormationTargetOptions;
pub use formation_targets::FormationTargets;
pub use heap_arrival_order::HeapArrivalOrder;
pub use i_arrival_order::IArrivalOrder;
pub use i_arrival_path::IArrivalPath;
pub use random_arrival_order::RandomArrivalOrder;
pub use spiral_arrival::{SpiralArrival, SpiralArrivalOptions};
pub use straight_arrival::StraightArrival;
pub use swaying_fall_arrival::SwayingFallArrival;
