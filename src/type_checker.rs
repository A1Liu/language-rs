use crate::syntax_tree::*;
use crate::util::*;
use std::collections::HashMap;

pub struct TypeChecker<'a, 'b>
where
    'b: 'a,
{
    buckets: &'a mut Buckets<'b>,
    type_table: HashMap<u32, &'a InferredType<'a>>,
    symbol_table: HashMap<u32, &'a InferredType<'a>>,
}

impl<'a, 'b> TypeChecker<'a, 'b>
where
    'b: 'a,
{
    pub fn new(buckets: &'a mut Buckets<'b>) -> Self {
        let type_table = HashMap::new();
        let symbol_table = HashMap::new();
        return Self {
            buckets,
            type_table,
            symbol_table,
        };
    }

    pub fn check_program(&mut self, program: &'b mut [Stmt<'b>]) -> Result<(), Error<'b>> {
        for stmt in program {
            self.check_stmt(stmt)?;
        }
        return Ok(());
    }

    pub fn check_stmt(&mut self, stmt: &'b mut Stmt<'b>) -> Result<(), Error<'b>> {
        match stmt {
            Stmt::Pass => return Ok(()),
            Stmt::End => return Ok(()),
            Stmt::Expr(expr) => {
                self.check_expr(expr)?;
                return Ok(());
            }
            Stmt::Assign { to, value } => {
                return Err(Error {
                    location: to.view.start..value.view.end,
                    message: "no assignments yet",
                })
            }
        }
    }

    pub fn check_expr(
        &mut self,
        expr: &'b mut Expr<'b>,
    ) -> Result<&'b InferredType<'b>, Error<'b>> {
        if expr.inferred_type != InferredType::Unknown {
            return Ok(&expr.inferred_type);
        }

        // match expr.tag {}
        return Ok(&expr.inferred_type);
    }
}
