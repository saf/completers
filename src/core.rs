//! Module for core elements of the completers application:
//! completions and completion providers (aka Completers).

use std::any;
use std::rc::Rc;

/// A trait representing a single completion.
///
/// Completions can form a tree structure, with each completion having
/// any number of child completions. This can model a file system
/// hierarchy, for example, but it can also be adapted for other
/// needs.
///
/// A completion will usually show up in the completion window as the
/// same text which is the result of the completion (i.e., the text
/// which is used if the completion is selected), but some completions
/// may override that, hence the distinction between `display_string`
/// and `result_string`.
pub trait Completion : any::Any {
    /// Returns the string which should be used as the completion.
    fn result_string(&self) -> String;

    /// Returns the string to be shown in the selection UI.
    fn display_string(&self) -> String {
        self.result_string()
    }

    /// Converts a completion to an `Any` reference.
    ///
    /// This is needed for technical reasons because concrete
    /// completers will have to down-cast `Completion` trait objects.
    fn as_any(&self) -> &any::Any;
}

/// The type of collections of completions passed from the completers
/// to the UI.
pub type Completions = Vec<Rc<Completion>>;

/// A trait for types which provide completions.
///
/// complete-rs can support multiple completion providers and switch
/// between them in run-time.
pub trait Completer {
    /// Returns the completions provided by this completer.
    fn completions(&self) -> Completions;

    /// Indicates if the completer can 'descend' into the given completion.
    ///
    /// Descending can be used to model a tree structure (e.g., a file
    /// system) or any other hierarchical structure.
    fn can_descend(&self, &Completion) -> bool {
        false
    }

    /// Descends into the given completion.
    ///
    /// This does not need to be implemented in any meaningful way if
    /// the completer always returns `false` from `can_descend`; hence,
    /// we provide a default implementation which does nothing.
    fn descend(&mut self, &Completion) {}

    /// Indicates if the completer can "ascend" from the current level.
    ///
    /// Ascending can be used to go back from a node we descended
    /// into, but it can also model going up a hierarchical structure
    /// from a point where the completer was first invoked, e.g.,
    /// going to the parent of the current directory.
    fn can_ascend(&self) -> bool {
        false
    }

    /// Ascends from the current state.
    ///
    /// We provide a default implementation which does nothing; it is
    /// OK for completers which always return `false` from the
    /// `can_ascend` method.
    ///
    /// A reasonable completer will support ascending from states it
    /// allows descending to.
    fn ascend(&mut self) {}
}
