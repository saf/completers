//! Module for core elements of the completers application:
//! completions and completion providers (aka Completers).

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
pub trait Completion : 'static {
    /// Returns the string which should be used as the completion.
    fn result_string(&self) -> &str;

    /// Returns the string to be shown in the selection UI.
    fn display_string(&self) -> &str {
        self.result_string()
    }

    /// Indicates if this node has any children.
    fn has_children(&self) -> bool {
        false
    }

    /// Returns a vector of the child's children.
    fn children(&self) -> Vec<Box<Completion>>;
}

/// A trait for types which provide completions.
///
/// complete-rs can support multiple completion providers and switch
/// between them in run-time.
pub trait Completer {
    /// Returns a slice containing the completions provided by this
    /// completer.
    fn completions(&self) -> &[Box<Completion>];
}
