#![deny(clippy::all)]
#![allow(clippy::arc_with_non_send_sync)]

use swc_common::{chain, comments::Comments, sync::Lrc, Mark, SourceMap};
use swc_ecma_visit::{Fold, VisitMut};

pub use self::{
    jsx::*,
    pure_annotations::pure_annotations,
    refresh::{options::RefreshOptions, refresh},
};

mod inferno_flags;
mod jsx;
mod pure_annotations;
mod refresh;
mod vnode_types;
mod atoms;

///
/// `top_level_mark` should be [Mark] passed to
/// [swc_ecma_transforms_base::resolver::resolver_with_mark].
///
///
///
/// # Note
///
/// This pass uses [swc_ecma_utils::HANDLER].
pub fn inferno<C>(
    cm: Lrc<SourceMap>,
    comments: Option<C>,
    mut options: Options,
    top_level_mark: Mark,
    unresolved_mark: Mark,
) -> impl Fold + VisitMut
where
    C: Comments + Clone,
{
    let Options { development, .. } = options;
    let development = development.unwrap_or(false);

    let refresh_options = options.refresh.take();

    chain!(
        refresh(
            development,
            refresh_options,
            cm.clone(),
            comments.clone(),
            top_level_mark
        ),
        jsx(
            cm,
            comments.clone(),
            options,
            top_level_mark,
            unresolved_mark
        ),
        pure_annotations(comments),
    )
}
