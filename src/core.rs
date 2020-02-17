//! Module for core elements of the completers application:
//! completions and completion providers (aka Completers).

use std::any;
use std::sync::Arc;

/// A trait representing a single completion.
///
/// A completion will usually show up in the completion window as the
/// same text which is the result of the completion (i.e., the text
/// which is used if the completion is selected), but some completions
/// may override that, hence the distinction between `display_string`
/// and `result_string`.
pub trait Completion: any::Any {
    /// Returns the string which should be used as the completion.
    fn result_string(&self) -> String;

    /// Returns the string to be shown in the selection UI.
    ///
    /// The default implementation is to show the same string as
    /// `result_string`.
    fn display_string(&self) -> String {
        self.result_string()
    }

    /// Returns the string to be analyzed during the search.
    ///
    /// The default implementation is to search in the same
    /// string as `result_string`.
    fn search_string(&self) -> String {
        self.result_string()
    }

    /// Converts a completion to an `Any` reference.
    ///
    /// This is needed for technical reasons because concrete
    /// completers will have to down-cast `Completion` trait objects.
    fn as_any(&self) -> &dyn any::Any;
}

/// The type of completions returned from completers.
///
/// This type aims to make it easier for completers to store
/// collections of completions internally and return them from the
/// `completions` routine. An alternative design would be to have
/// completers store the concrete completion types internally and
/// returning references to them from `completions`, but that would
/// require building separate collections of those references. With
/// this type in place, completers can build their collections of
/// completions as collections of Arcs to `core::Completion` trait
/// objects and return references to those collections from their
/// `completions` methods.
pub type CompletionBox = Arc<dyn Completion + Sync + Send>;

/// A trait for types which provide completions.
///
/// complete-rs can support multiple completion providers and switch
/// between them in run-time.
pub trait Completer {
    /// Returns the name of the completer.
    fn name(&self) -> String;

    /// Returns the completions provided by this completer.
    ///
    /// Completers are expected to store the collection of their
    /// completions within their structure, and return a reference to
    /// the relevant slice from this method.
    fn completions(&self) -> &[CompletionBox];

    /// Indicates if fetching completions is finished.
    ///
    /// A completer may return `false` from this method to indicate
    /// that there may be more completions in the future. This is
    /// useful if fetching all completions may take a long time.
    fn fetching_completions_finished(&self) -> bool {
        true
    }

    /// Requests the completer to update its collection of completions.
    ///
    /// The framework will call this until the completer returns `true`
    /// from `fetching_completions_finished`.
    ///
    /// The default implementation is to do nothing; this is
    /// appropriate for completers which generate all their
    /// completions at once.
    fn fetch_completions(&mut self) {}

    /// Descends into the given completion if possible, yielding a new
    /// completer. Returns None if descending is not possible for the
    /// completion.
    ///
    /// A completer may return a new completer of the same type or
    /// another type.
    ///
    /// The default implementation returns None for any completion,
    /// which means that descending is not possible for any
    /// completion.
    fn descend(&self, _: &dyn Completion) -> Option<Box<dyn Completer>> {
        None
    }

    /// Ascends from the current state -- moves "up" in the
    /// hierarchical structure.
    ///
    /// Ascending is only meaningful for completers which are not the
    /// result of descending into a completion. If a completer is the
    /// result of descending into a completion, the framework will
    /// handle ascending from it by moving to the completer which
    /// spawned that completion.
    ///
    /// A completer may return a new completer of the same or
    /// different type than itself, or return None to indicate that
    /// ascending from the current completer is not possible.
    fn ascend(&self) -> Option<Box<dyn Completer>> {
        None
    }
}
