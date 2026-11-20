//! `FormationElements`: a formation turned into time-keyed elements of a session (C#
//! `FormationElements`, made by `BindFormation`).

use std::rc::Rc;

use crate::drawing_session::DrawingSession;
use crate::formations::FormationMember;
use crate::models::binding_combine::SessionDriver;
use crate::models::{AnimationBinding, ElementId};

/// The binding's driver: poses the travelling elements of every member travelling at a time.
pub(crate) fn driver(members: Rc<Vec<FormationMember>>, travelling: Rc<Vec<Vec<ElementId>>>) -> Rc<dyn SessionDriver> {
    Rc::new(FormationDriver { members, travelling })
}

pub(crate) struct FormationDriver {
    pub(crate) members: Rc<Vec<FormationMember>>,
    pub(crate) travelling: Rc<Vec<Vec<ElementId>>>,
}

impl SessionDriver for FormationDriver {
    fn apply(&self, session: &mut DrawingSession, seconds: f64) {
        for (member, parts) in self.members.iter().zip(self.travelling.iter()) {
            if parts.is_empty() || !member.is_travelling_at(seconds) {
                continue;
            }
            let Some(path) = member.path() else { continue };
            let pose = path.get_pose_at(seconds).to_transform();
            for part in parts {
                if let Some(e) = session.find_element_mut(*part) {
                    e.set_transform(pose);
                }
            }
        }
    }
}

/// The elements of a bound formation (C# `FormationElements`): per member (in the order given)
/// its resting element and its travelling elements, the arrival order, and the one binding that
/// poses the travellers.
#[derive(Clone)]
pub struct FormationElements {
    pub(crate) members: Rc<Vec<FormationMember>>,
    pub(crate) resting: Vec<ElementId>,
    pub(crate) travelling: Rc<Vec<Vec<ElementId>>>,
    pub(crate) order: Vec<usize>,
    pub(crate) binding: AnimationBinding,
}

impl FormationElements {
    /// The number of members (C# `Count`).
    pub fn count(&self) -> usize {
        self.members.len()
    }

    /// The members, in the order given (C# `Members`).
    pub fn members(&self) -> &[FormationMember] {
        &self.members
    }

    /// The members' indices in arrival order - a stable sort by arrival time (C# `ArrivalOrder`).
    pub fn arrival_order(&self) -> &[usize] {
        &self.order
    }

    /// The binding that poses the travelling elements (C# `Binding`).
    pub fn binding(&self) -> &AnimationBinding {
        &self.binding
    }

    /// Member `index`'s resting element (C# `GetResting`).
    pub fn get_resting(&self, index: usize) -> Option<ElementId> {
        self.resting.get(index).copied()
    }

    /// Member `index`'s travelling elements (C# `GetTravelling`; empty for a member that appears).
    pub fn get_travelling(&self, index: usize) -> Option<&[ElementId]> {
        self.travelling.get(index).map(|v| v.as_slice())
    }

    /// How many members are travelling at a time (C# `GetTravellingCount`).
    pub fn get_travelling_count(&self, seconds: f64) -> usize {
        self.members.iter().filter(|m| m.is_travelling_at(seconds)).count()
    }
}

impl std::fmt::Debug for FormationElements {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FormationElements").field("count", &self.count()).field("binding", &self.binding).finish()
    }
}
