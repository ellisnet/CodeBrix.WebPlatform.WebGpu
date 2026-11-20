//! `FormationMember`: one member of a formation - its way in, or just its arrival time (C#
//! `FormationMember`).

use std::rc::Rc;

use crate::formations::{FormationTarget, IArrivalPath};

/// One member of a formation (C# `FormationMember`): either it TRAVELS (an [`IArrivalPath`]) or
/// it just APPEARS at its arrival time.
#[derive(Clone)]
pub struct FormationMember {
    path: Option<Rc<dyn IArrivalPath>>,
    arrival_time: f64,
}

impl FormationMember {
    /// A member that travels along a path (C# `FormationMember(IArrivalPath)`).
    pub fn with_path(path: Rc<dyn IArrivalPath>) -> FormationMember {
        let arrival_time = path.arrival_time();
        FormationMember { path: Some(path), arrival_time }
    }

    /// A member that appears at a time (C# `FormationMember(double arrivalTime)`). The error says
    /// what to do with a time that is not finite.
    pub fn new(arrival_time: f64) -> Result<FormationMember, String> {
        if !arrival_time.is_finite() {
            return Err("The arrival time must be a finite number of seconds.".to_string());
        }
        Ok(FormationMember { path: None, arrival_time })
    }

    /// The way in (None for a member that appears) (C# `Path`).
    pub fn path(&self) -> Option<&Rc<dyn IArrivalPath>> {
        self.path.as_ref()
    }

    /// When it arrives (C# `ArrivalTime`).
    pub fn arrival_time(&self) -> f64 {
        self.arrival_time
    }

    /// When it sets off - its arrival time when it does not travel (C# `StartTime`).
    pub fn start_time(&self) -> f64 {
        self.path.as_ref().map_or(self.arrival_time, |p| p.start_time())
    }

    /// True while it travels: from its path's start until (not at) its arrival (C#
    /// `IsTravellingAt`).
    pub fn is_travelling_at(&self, seconds: f64) -> bool {
        match &self.path {
            Some(p) => seconds >= p.start_time() && seconds < self.arrival_time,
            None => false,
        }
    }

    /// Members from targets and their arrival times, each with the path `create_path(target,
    /// arrival_time, k)` makes (C# `FromTargets`). The error says what to do when the counts
    /// differ.
    pub fn from_targets(targets: &[FormationTarget], arrival_times: &[f64], mut create_path: impl FnMut(FormationTarget, f64, usize) -> Rc<dyn IArrivalPath>) -> Result<Vec<FormationMember>, String> {
        if targets.len() != arrival_times.len() {
            return Err("There must be one arrival time per target.".to_string());
        }
        Ok(targets.iter().zip(arrival_times).enumerate().map(|(k, (t, a))| FormationMember::with_path(create_path(*t, *a, k))).collect())
    }
}

impl std::fmt::Debug for FormationMember {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FormationMember").field("arrival_time", &self.arrival_time).field("start_time", &self.start_time()).field("travels", &self.path.is_some()).finish()
    }
}
