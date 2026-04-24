use dol_expr::expr::{DeleteNode, InsertNode, QueryNode, UpdateNode, UpsertNode};

#[derive(Debug, Clone)]
pub enum Statement {
    Query(QueryNode),
    Insert(InsertNode),
    Update(UpdateNode),
    Delete(DeleteNode),
    Upsert(UpsertNode),
    Raw(String),
}
