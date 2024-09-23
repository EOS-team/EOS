// SPDX-License-Identifier: GPL-2.0

use kernel::{
    bindings,
    prelude::*,
    security,
    sync::{Mutex, Ref},
};

use crate::{
    node::NodeRef,
    thread::{BinderError, BinderResult},
};

struct Manager {
    node: Option<NodeRef>,
    uid: Option<bindings::kuid_t>,
}

pub(crate) struct Context {
    manager: Mutex<Manager>,
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}

impl Context {
    pub(crate) fn new() -> Result<Ref<Self>> {
        Ref::try_new_and_init(
            Self {
                // SAFETY: Init is called below.
                manager: unsafe {
                    Mutex::new(Manager {
                        node: None,
                        uid: None,
                    })
                },
            },
            |mut ctx| {
                // SAFETY: `manager` is also pinned when `ctx` is.
                let manager = unsafe { ctx.as_mut().map_unchecked_mut(|c| &mut c.manager) };
                kernel::mutex_init!(manager, "Context::manager");
            },
        )
    }

    pub(crate) fn set_manager_node(&self, node_ref: NodeRef) -> Result {
        let mut manager = self.manager.lock();
        if manager.node.is_some() {
            return Err(Error::EBUSY);
        }
        security::binder_set_context_mgr(&node_ref.node.owner.task)?;

        // TODO: Get the actual caller id.
        let caller_uid = bindings::kuid_t::default();
        if let Some(ref uid) = manager.uid {
            if uid.val != caller_uid.val {
                return Err(Error::EPERM);
            }
        }

        manager.node = Some(node_ref);
        manager.uid = Some(caller_uid);
        Ok(())
    }

    pub(crate) fn unset_manager_node(&self) {
        let node_ref = self.manager.lock().node.take();
        drop(node_ref);
    }

    pub(crate) fn get_manager_node(&self, strong: bool) -> BinderResult<NodeRef> {
        self.manager
            .lock()
            .node
            .as_ref()
            .ok_or_else(BinderError::new_dead)?
            .clone(strong)
    }
}
