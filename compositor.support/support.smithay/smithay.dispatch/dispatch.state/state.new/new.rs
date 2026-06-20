// The `Dispatch` factory moved to wire.base (`new_dispatch`) as part of the
// P2 flip: building a concrete `Dispatch` calls `create_global::<Dispatch>` for
// every protocol, which needs `Dispatch: GlobalDispatch<…>` — provable only
// where `delegate_dispatch2!(Dispatch)` lives (document/SMITHAY_DECOUPLING.md).
// This crate is retained as a workspace member but no longer hosts the factory.
