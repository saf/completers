use std::cmp;

use crate::config::*;
use crate::core;
use crate::scoring;

#[derive(Clone)]
pub struct CompletionWithScore {
    pub completion: core::CompletionBox,
    pub score: scoring::Score,
}

/// Compare scored completions so that more points go first.
impl Ord for CompletionWithScore {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.score.cmp(&other.score).reverse()
    }
}

impl PartialEq for CompletionWithScore {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl PartialOrd for CompletionWithScore {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for CompletionWithScore {}

struct CompleterView {
    /// The completer which provides the propositions for this view.
    pub completer: Box<dyn core::Completer>,

    /// The index of the first completion which is currently
    /// displayed.
    pub view_offset: usize,

    /// The index of the currently selected completion.
    pub selection: usize,

    /// The current query for this completer.
    pub query: String,

    /// All completions which have been fetched so far.
    ///
    /// This is not affected by the query.
    all_completions: Vec<core::CompletionBox>,

    /// Completions for the current query.
    ///
    /// This is sorted by score, so that completions with the highest
    /// score are at the beginning of the vector.
    scored_completions: Vec<CompletionWithScore>,
}

impl CompleterView {
    pub fn new(completer: Box<dyn core::Completer>) -> CompleterView {
        CompleterView {
            completer: completer,
            view_offset: 0,
            selection: 0,
            query: "".to_string(),
            all_completions: Vec::new(),
            scored_completions: Vec::new(),
        }
    }

    fn selected_completion(&self) -> Option<core::CompletionBox> {
        self.scored_completions
            .get(self.selection)
            .map(|sc| sc.completion.clone())
    }

    pub fn select_previous(&mut self) {
        self.selection = self.selection.saturating_sub(1);
        if self.selection < self.view_offset {
            self.view_offset = self.view_offset - 1;
        }
    }

    pub fn select_next(&mut self) {
        let completions_count = self.scored_completions.len();
        self.selection = cmp::min(self.selection + 1, completions_count.saturating_sub(1));
        if self.selection >= self.view_offset + CHOOSER_HEIGHT {
            self.view_offset = self.view_offset + 1;
        }
    }

    pub fn previous_page(&mut self) {
        self.selection = self.selection.saturating_sub(CHOOSER_HEIGHT);
        if self.selection < self.view_offset {
            self.view_offset = self.selection;
        }
    }

    pub fn next_page(&mut self) {
        let completions_count = self.scored_completions.len();
        self.selection = cmp::min(self.selection + CHOOSER_HEIGHT, completions_count - 1);
        if self.selection >= self.view_offset + CHOOSER_HEIGHT {
            self.view_offset = self.selection.saturating_sub(CHOOSER_HEIGHT - 1);
        }
    }

    pub fn select_first(&mut self) {
        self.selection = 0;
        self.view_offset = 0;
    }

    pub fn select_last(&mut self) {
        let completions_count = self.scored_completions.len();
        self.selection = completions_count - 1;
        self.view_offset = self.selection.saturating_sub(CHOOSER_HEIGHT - 1);
    }

    fn update_query(&mut self, new_query: String) {
        self.selection = 0;
        self.view_offset = 0;
        self.query = new_query;
        self.scored_completions = self.compute_scores(self.all_completions.clone())
    }

    fn fetch_completions(&mut self) {
        let new_completions = self.completer.fetch_completions();
        let new_completions_scored = self.compute_scores(new_completions.clone());
        self.all_completions.extend(new_completions.into_iter());
        let existing_completions_scored = self.scored_completions.drain(..);
        self.scored_completions =
            itertools::merge(existing_completions_scored, new_completions_scored).collect();
    }

    fn compute_scores(&self, completions: Vec<core::CompletionBox>) -> Vec<CompletionWithScore> {
        let scoring_settings = scoring::ScoringSettings {
            letter_match: 1,
            word_start_bonus: 2,
            subsequent_bonus: 3,
        };
        let mut scored_completions = completions
            .into_iter()
            .filter(|c| scoring::subsequence_match(&self.query, &c.search_string()))
            .map(|c| CompletionWithScore {
                completion: c.clone(),
                score: scoring::score(&c.search_string(), &self.query, &scoring_settings),
            })
            .collect::<Vec<_>>();
        scored_completions.sort();
        scored_completions
    }

    fn completions(&self) -> Vec<CompletionWithScore> {
        self.scored_completions.clone()
    }
}

/// A structure representing a single stack of completers.
///
/// The stack may be expanded by descending into the selected
/// completer. The completer stack is never empty.
struct CompleterStack {
    stack: Vec<CompleterView>,
}

impl CompleterStack {
    pub fn new(completer: Box<dyn core::Completer>) -> CompleterStack {
        CompleterStack {
            stack: vec![CompleterView::new(completer)],
        }
    }

    pub fn top(&self) -> &CompleterView {
        self.stack.last().unwrap()
    }

    pub fn top_mut(&mut self) -> &mut CompleterView {
        self.stack.last_mut().unwrap()
    }

    /// Descends into the selected completion.
    ///
    /// Returns `true` if we descended anywhere, `false` if we stayed in the same view.
    fn descend(&mut self) -> bool {
        if let Some(scb) = self.top().selected_completion() {
            if let Some(descended_completer) = self.top().completer.descend(&*scb) {
                let mut new_level = CompleterView::new(descended_completer);
                new_level.fetch_completions();
                self.stack.push(new_level);
                return true;
            }
        }
        false
    }

    fn ascend(&mut self) {
        if self.stack.len() == 1 {
            if let Some(new_completer) = self.top().completer.ascend() {
                let mut new_level = CompleterView::new(new_completer);
                new_level.fetch_completions();
                self.stack[0] = new_level;
            }
        } else {
            self.stack.pop();
        }
    }
}

/// A structure representing the entire model of the data necessary to
/// handle multiple stacks of completers.
///
/// The model consists of a collection of completer stacks which is
/// initialized with a collection of completers. Completers may have a
/// levelled structure, allowing "descending" and "ascending" into
/// different completers; this is represented by the stacks of
/// completers, one for each of the initial completers.
pub struct Model {
    /// The collection of tabs (completer stacks).
    stacks: Vec<CompleterStack>,

    /// The index within `stacks` of the stack which is currently selected.
    selection: usize,

    /// The current query.
    query: String,
}

impl Model {
    pub fn new(completers: Vec<Box<dyn core::Completer>>) -> Model {
        let mut stacks = vec![];
        for c in completers {
            stacks.push(CompleterStack::new(c));
        }
        Model {
            stacks: stacks,
            selection: 0,
            query: "".to_string(),
        }
    }

    fn current_stack(&self) -> &CompleterStack {
        &self.stacks[self.selection]
    }

    fn current_stack_mut(&mut self) -> &mut CompleterStack {
        &mut self.stacks[self.selection]
    }

    fn current_view(&self) -> &CompleterView {
        self.current_stack().top()
    }

    fn current_view_mut(&mut self) -> &mut CompleterView {
        self.current_stack_mut().top_mut()
    }

    pub fn completer_name(&self) -> String {
        self.current_view().completer.name()
    }

    pub fn completions(&self) -> Vec<CompletionWithScore> {
        self.current_view().completions()
    }

    pub fn get_selected_result(&self) -> Option<String> {
        self.current_view()
            .selected_completion()
            .map(|c| c.result_string())
    }

    pub fn view_offset(&self) -> usize {
        self.current_view().view_offset
    }

    pub fn selection(&self) -> usize {
        self.current_view().selection
    }

    pub fn select_previous(&mut self) {
        self.current_view_mut().select_previous();
    }

    pub fn select_next(&mut self) {
        self.current_view_mut().select_next();
    }

    pub fn previous_page(&mut self) {
        self.current_view_mut().previous_page();
    }

    pub fn next_page(&mut self) {
        self.current_view_mut().next_page();
    }

    pub fn select_first(&mut self) {
        self.current_view_mut().select_first();
    }

    pub fn select_last(&mut self) {
        self.current_view_mut().select_last();
    }

    fn update_query(&mut self) {
        let query: String = self.query.clone();
        self.current_view_mut().update_query(query);
    }

    pub fn query_backspace(&mut self) {
        self.query.pop();
        self.update_query();
    }

    pub fn query_append(&mut self, ch: char) {
        self.query.push(ch);
        self.update_query()
    }

    pub fn query_set(&mut self, query: &str) {
        self.query = query.to_string();
        self.update_query()
    }

    pub fn query(&self) -> String {
        self.query.clone()
    }

    pub fn descend(&mut self) {
        let descended = self.current_stack_mut().descend();
        if descended {
            self.query_set("");
        }
    }

    pub fn ascend(&mut self) {
        self.current_stack_mut().ascend()
    }

    pub fn next_tab(&mut self) {
        // We preserve the query when switching tabs in order
        // to retain the initial query when the user switches
        // between tabs at the beginning.
        self.selection = (self.selection + 1) % self.stacks.len();
        self.update_query();
    }

    pub fn start_fetching_completions(&mut self) {
        for stack in &mut self.stacks {
            stack.top_mut().fetch_completions();
        }
    }

    pub fn fetch_completions(&mut self) {
        self.current_view_mut().fetch_completions();
    }

    pub fn fetching_completions_finished(&self) -> bool {
        self.current_view()
            .completer
            .fetching_completions_finished()
    }
}
