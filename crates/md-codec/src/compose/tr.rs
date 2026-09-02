//! Taproot lowering. STUB until its tests exist (Task 3 replaces this file).

use super::{ComposeError, Composed, PathList, SlotOrigin};

pub(super) fn lower_tr(
    list: &PathList,
    declared: &[Option<SlotOrigin>],
) -> Result<Composed, ComposeError> {
    let _ = (list, declared);
    unimplemented!("the taproot lowering lands with its tests")
}
