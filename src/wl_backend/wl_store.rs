use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

use snafu::{prelude::*, Location};

use crate::wl_backend::{WlGenericId, WlHead, WlMode};

use super::{WlBaseHead, WlBaseMode};

pub trait ForeignId {
    type Id: std::fmt::Debug + Clone + PartialEq + Eq + Hash;

    fn foreign_id(&self) -> Self::Id;
}

#[derive(Clone, Debug)]
pub struct StoreHead<H, I>
where
    H: ForeignId<Id = I>,
{
    pub base: WlBaseHead,
    pub(crate) current_mode: Option<I>,
    modes: Vec<I>,
    id: WlGenericId,
    pub(crate) foreign_head: H,
    _foreign_id: I,
}

#[derive(Clone, Debug)]
pub struct StoreMode<M, I>
where
    M: ForeignId<Id = I>,
{
    pub base: WlBaseMode,
    id: WlGenericId,
    pub(crate) foreign_mode: M,
    _foreign_id: I,
}

#[derive(Clone, Debug)]
pub struct WlStore<H, M, I>
where
    H: ForeignId<Id = I>,
    M: ForeignId<Id = I>,
{
    /// A Mapping from [`WlHead`]-Ids to [`WlHead`]s
    // heads: HashMap<I, Rc<RefCell<StoreHead<I>>>>,
    heads: HashMap<I, StoreHead<H, I>>,
    /// A Mapping from [`WlMode`]-Ids to [`WlMode`]s
    // modes: HashMap<I, Rc<RefCell<StoreMode<I>>>>,
    modes: HashMap<I, StoreMode<M, I>>,
    /// A Mapping from [`WlMode`]-Ids to [`WlHead`]-Ids
    ///
    /// The [`WlMode`] from the key-field belongs to the [`WlHead`] in the value-field
    mode_id_head_id: HashMap<I, I>,
    store_key_to_foreign_key: HashMap<WlGenericId, I>,
    id_counter: usize,
}

impl<H, M, I> Default for WlStore<H, M, I>
where
    H: ForeignId<Id = I>,
    M: ForeignId<Id = I>,
{
    fn default() -> Self {
        Self {
            heads: Default::default(),
            modes: Default::default(),
            mode_id_head_id: Default::default(),
            store_key_to_foreign_key: Default::default(),
            id_counter: Default::default(),
        }
    }
}

impl<H, M, I> WlStore<H, M, I>
where
    H: ForeignId<Id = I>,
    M: ForeignId<Id = I>,
    I: std::fmt::Debug + Clone + PartialEq + Eq + Hash,
{
    pub fn export(&self) -> Result<VecDeque<WlHead>, WlStoreError<I>> {
        self.heads
            .values()
            .map(|head| self.export_head(head))
            .collect()
    }

    pub fn heads_count(&self) -> usize {
        self.heads.len()
    }

    pub fn head(&self, head_id: I) -> Result<&StoreHead<H, I>, WlStoreError<I>> {
        self.heads
            .get(&head_id)
            .context(HeadNotFoundCtx { head_id })
    }
    pub fn mode(&self, mode_id: I) -> Result<&StoreMode<M, I>, WlStoreError<I>> {
        self.modes
            .get(&mode_id)
            .context(ModeNotFoundCtx { mode_id })
    }
    pub fn mode_store_key(
        &self,
        mode_id: WlGenericId,
    ) -> Result<&StoreMode<M, I>, WlStoreError<I>> {
        let mode_id = self
            .store_key_to_foreign_key
            .get(&mode_id)
            .context(UnknownStoreKeyCtx { key: mode_id })?;
        self.mode(mode_id.clone())
    }
    pub fn head_store_key(
        &self,
        head_id: WlGenericId,
    ) -> Result<&StoreHead<H, I>, WlStoreError<I>> {
        let head_id = self
            .store_key_to_foreign_key
            .get(&head_id)
            .context(UnknownStoreKeyCtx { key: head_id })?;
        self.head(head_id.clone())
    }

    pub fn head_mut(&mut self, head_id: I) -> Result<&mut StoreHead<H, I>, WlStoreError<I>> {
        // let head_id = head_id.clone();
        self.heads
            .get_mut(&head_id)
            .context(HeadNotFoundCtx { head_id })
    }
    pub fn mode_mut(&mut self, mode_id: I) -> Result<&mut StoreMode<M, I>, WlStoreError<I>> {
        // let mode_id = mode_id.clone();
        self.modes
            .get_mut(&mode_id)
            .context(ModeNotFoundCtx { mode_id })
    }

    pub fn insert_head(&mut self, foreign_head: H) {
        let foreign_head_id = foreign_head.foreign_id();
        let store_head = StoreHead {
            base: Default::default(),
            current_mode: Default::default(),
            modes: Default::default(),
            id: self.new_store_id(),
            foreign_head,
            _foreign_id: foreign_head_id.clone(),
        };
        self.store_key_to_foreign_key
            .insert(store_head.id, foreign_head_id.clone());
        self.heads.insert(foreign_head_id, store_head);
    }

    pub fn insert_mode(&mut self, foreign_head_id: I, foreign_mode: M) {
        let foreign_mode_id = foreign_mode.foreign_id();
        let store_mode = StoreMode {
            base: Default::default(),
            id: self.new_store_id(),
            foreign_mode,
            _foreign_id: foreign_mode_id.clone(),
        };
        self.store_key_to_foreign_key
            .insert(store_mode.id, foreign_mode_id.clone());
        self.heads
            .entry(foreign_head_id.clone())
            .and_modify(|head| head.modes.push(foreign_mode_id.clone()));

        self.mode_id_head_id
            .insert(foreign_mode_id.clone(), foreign_head_id);
        self.modes.insert(foreign_mode_id, store_mode);
    }
    /// This function removes all occurences of the provided `Id` of the mode in [`WlStore`].
    pub fn remove_mode(&mut self, mode_id: &I) -> Result<(), WlStoreError<I>> {
        // the Id of the head the mode belongs to
        let head_id = self.mode_id_head_id.remove(mode_id).ok_or(
            ModeNotFoundCtx {
                mode_id: mode_id.clone(),
            }
            .build(),
        )?;
        let head = self
            .heads
            .get_mut(&head_id)
            .ok_or(HeadNotFoundCtx { head_id }.build())?;

        if let Some(c_mode_id) = &head.current_mode {
            if *c_mode_id == *mode_id {
                head.current_mode = None;
            }
        }
        head.modes.retain(|id| *id != *mode_id);

        Ok(())
    }
    pub fn remove_head(&mut self, head_id: &I) {
        self.heads.remove(head_id);
    }

    fn export_head(&self, head: &StoreHead<H, I>) -> Result<WlHead, WlStoreError<I>> {
        let StoreHead {
            base,
            current_mode: store_head_current_mode,
            modes,
            id,
            foreign_head: _,
            _foreign_id: _,
        } = head;

        let mut current_mode: Option<WlMode> = None;
        if let Some(current_mode_id) = store_head_current_mode {
            let store_mode = self.mode(current_mode_id.clone())?;

            current_mode = Some(WlMode {
                base: store_mode.base,
                id: store_mode.id,
            });
        }

        let modes = modes
            .iter()
            .map(|mi| self.mode(mi.clone()))
            .filter_map(|r| r.ok())
            .map(|m| WlMode {
                base: m.base,
                id: m.id,
            })
            .collect();

        let wl_head = WlHead {
            base: base.clone(),
            current_mode,
            modes,
            id: *id,
        };
        Ok(wl_head)
    }

    // fn export_mode(&self, mode_id: &I) -> Result<WlMode, StoreError<I>> {
    //     let StoreMode { inner, wl_mode } = self.mode(mode_id)?;
    //     Ok(tmode_from(*inner, wl_mode.clone()))
    // }

    fn new_store_id(&mut self) -> WlGenericId {
        self.id_counter += 1;
        WlGenericId(self.id_counter)
    }
}

#[derive(Debug, Snafu)]
#[snafu(context(suffix(Ctx)))]
#[snafu(visibility(pub(crate)))]
pub enum WlStoreError<I: std::fmt::Debug> {
    #[snafu(display("[{location}] Cannot find head in store: {head_id:?}"))]
    HeadNotFound { location: Location, head_id: I },
    #[snafu(display("[{location}] Cannot find mode in store: {mode_id:?}"))]
    ModeNotFound { location: Location, mode_id: I },
    #[snafu(display("[{location}] Cannot find store key: {key:?}"))]
    UnknownStoreKey {
        location: Location,
        key: WlGenericId,
    },
}
