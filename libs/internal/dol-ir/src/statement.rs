use dol_expr::expr::{MutateNode, SelectNode};

#[derive(Debug, Clone)]
pub enum Statement {
    Select(SelectNode),
    Insert(MutateNode),
    Update(MutateNode),
    Delete(MutateNode),
    Upsert(MutateNode),
    Raw(String),
}
