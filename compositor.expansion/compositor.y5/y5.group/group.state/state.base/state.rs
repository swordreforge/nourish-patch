use std::{collections::HashMap, sync::Arc};

use compositor_y5_group_surface_base::ui::GroupUi;
use uuid::Uuid;
use compositor_monitor_compositor_iced_base::{HandleId, IcedHandle};
use compositor_support_system_storage_token_base::base::{Token, TokenMut};

/// The window-grouping slot. Token lives beside its type (cycle-free) so the core
/// focus accessor can resolve it; owned/mutated by GroupSystem.
pub static GROUP: Token<GroupState> = Token::new();
pub static GROUP_MUT: TokenMut<GroupState> = TokenMut::new(&GROUP);

pub struct GroupState {
    // Mapping between a window UUID to its corresponding group.
    pub window: HashMap<uuid::Uuid, Arc<uuid::Uuid>>,
    pub group: HashMap<uuid::Uuid, Group>,
}

pub enum IcedInvalidation {
    New,
    BBOX,
    Destroy(HandleId),
}

impl GroupVisibility {
    pub fn ui_mode(&self) -> compositor_y5_group_surface_base::mode::Mode {
        match self {
            GroupVisibility::Collapse(_) => {
                return compositor_y5_group_surface_base::mode::Mode::Collapse;
            }
            GroupVisibility::Visible(_) => {
                return compositor_y5_group_surface_base::mode::Mode::Show;
            }
        }
    }
    pub fn id(&self) -> Option<HandleId> {
        match self {
            GroupVisibility::Collapse(iced_handle) => iced_handle.and_then(|w| Some(w.id)),
            GroupVisibility::Visible(iced_handle) => iced_handle.and_then(|w| Some(w.id)),
        }
    }

    pub fn handle(self) -> Option<IcedHandle<GroupUi>> {
        match self {
            GroupVisibility::Collapse(iced_handle) => iced_handle,
            GroupVisibility::Visible(iced_handle) => iced_handle,
        }
    }

    pub fn with_handle(&self, handle: IcedHandle<GroupUi>) -> GroupVisibility {
        match self {
            GroupVisibility::Collapse(None) => GroupVisibility::Collapse(Some(handle)),
            GroupVisibility::Visible(None) => GroupVisibility::Visible(Some(handle)),
            _ => {
                panic!("Unexpected group state when calling GroupVisibility::set");
            }
        }
    }
    pub fn without_handle(&self) -> GroupVisibility {
        match self {
            GroupVisibility::Collapse(Some(_)) => GroupVisibility::Collapse(None),
            GroupVisibility::Visible(Some(_)) => GroupVisibility::Visible(None),
            _ => {
                panic!("Unexpected group state when calling GroupVisibility::set");
            }
        }
    }
}
impl GroupState {
    pub fn new() -> Self {
        return Self {
            window: HashMap::new(),
            group: HashMap::new(),
        };
    }
    pub fn get_mut(&mut self, group_uuid: uuid::Uuid) -> Option<&mut Group> {
        self.group.get_mut(&group_uuid)
    }

    pub fn set(
        &mut self,
        window_uuid: &Vec<uuid::Uuid>,
        group_uuid: Option<Option<uuid::Uuid>>,
    ) -> HashMap<uuid::Uuid, IcedInvalidation> {
        let mut iced_invalidation = HashMap::new();
        let mut additionally_removed: Vec<uuid::Uuid> = vec![];

        // Remove and invalidate the groups
        {
            let mut group_removals: HashMap<uuid::Uuid, Vec<&uuid::Uuid>> = HashMap::new();

            // if the window is currently assigned a group, first call remove for the window
            'group_removals_add: for window in window_uuid {
                let window_group = self.window.remove(window);
                let Some(window_group) = window_group else {
                    continue 'group_removals_add;
                };
                if !group_removals.contains_key(window_group.as_ref()) {
                    let window_group_uuid = window_group.as_ref().clone();
                    group_removals.insert(window_group_uuid, vec![]);
                }

                let w = group_removals
                    .get_mut(window_group.as_ref())
                    .expect("upsert group");
                w.push(window);
            }

            self.group.retain(|group_id, group| {
                let Some(w) = group_removals.get(group_id) else {
                    return true;
                };

                group.window.retain(|ww| !w.contains(&ww));

                if group.window.len() <= 1 {
                    for w in &group.window {
                        additionally_removed.push(w.clone());
                    }

                    if let Some(iced_handle) = group.Visibility.id() {
                        iced_invalidation
                            .insert(group.id.clone(), IcedInvalidation::Destroy(iced_handle));
                    }

                    return false;
                } else {
                    // Make sure groups are invalidated when they are edited.
                    iced_invalidation.insert(group.id.clone(), IcedInvalidation::BBOX);
                }

                return true;
            });
        }

        for w in additionally_removed {
            self.window.remove(&w);
        }

        let Some(target_group) = group_uuid else {
            return iced_invalidation;
        };

        let (target_id, mut target_group, existing) = 'target_group: {
            if let Some(group_uuid) = target_group {
                iced_invalidation.insert(
                    group_uuid.clone(),
                    IcedInvalidation::BBOX,
                );

                // Expect the group to exist
                break 'target_group 'group: {
                    if let Some(group) = self.group.get(&group_uuid) {
                        break 'group Some((group_uuid, group.clone(), true));
                    }

                    None
                }
                .expect("Group to exist");
            } else {
                let group = Group::default();
                let id = group.id;
                self.group.insert(id, group.clone());
                iced_invalidation.insert(
                    id,
                    IcedInvalidation::New,
                );
                break 'target_group (id, group, false);
            };
        };

        let target_group_id = Arc::new(target_group.id.clone());

        // Add the windows to the group
        for window_uuid in window_uuid {
            self.window
                .insert(window_uuid.clone(), target_group_id.clone());
            target_group.window.push(window_uuid.clone());
        }

        self.group.insert(target_id, target_group);

        iced_invalidation
    }
}

#[derive(Clone)]
pub struct Group {
    pub id: uuid::Uuid,
    pub window: Vec<uuid::Uuid>,
    pub name: String,
    pub Visibility: GroupVisibility,
}

#[derive(Clone)]
pub enum GroupVisibility {
    Collapse(Option<IcedHandle<GroupUi>>),
    Visible((Option<IcedHandle<GroupUi>>)),
}

impl Default for Group {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::now_v7(),
            window: Default::default(),
            name: String::from("Group"),
            Visibility: GroupVisibility::Visible(None),
        }
    }
}
