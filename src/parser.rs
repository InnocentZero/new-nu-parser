use crate::compiler::{Compiler, RollbackPoint, Span};
use crate::errors::{Severity, SourceError};
use crate::lexer::{Token, Tokens};

use tracy_client::span;

pub struct Parser {
    pub compiler: Compiler,
    tokens: Tokens,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParamsId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InOutTypesId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CallId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ListId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TableId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RecordId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MatchId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeArgsId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineId(pub usize);

// TODO(bumpalo_rewrite): Fill with all the possible BlockEntities
// NOTE(bumpalo_rewrite): See Parser::block() for the entity types
#[derive(Debug, Clone)]
pub enum BlockEntities<'a> {
    Def(&'a Def<'a>),
    Let(&'a Let<'a>),
    While(&'a While<'a>),
    For(&'a For<'a>),
    Loop(&'a Loop<'a>),
    Return(&'a Return<'a>),
    Continue(&'a Continue),
    Break(&'a Break),
    Alias(&'a Alias),
    Extern(&'a Extern),
    PipelineOrExprOrAssign(PipelineOrExprOrAssign<'a>),
    Statement(PipelineOrExprOrAssign<'a>),
}

// NOTE(bumpalo_rewrite): This becomes the root of the AST
// TODO(bumpalo_rewrite): Change to the generic enum that block may contain
#[derive(Debug, Clone)]
pub struct Block<'a> {
    pub span_start: usize,
    pub span_end: usize,
    pub nodes: Vec<BlockEntities<'a>>,
}

// HACK(bumpalo_rewrite): not everything has to be pub
// TODO(bumpalo_rewrite): Finally uncomment this
#[derive(Debug, Clone)]
pub struct Def<'a> {
    pub span_start: usize,
    pub span_end: usize,
    // TODO(bumpalo_rewrite): Replace with a VarDecl type
    pub name: NodeId,
    // TODO(bumpalo_rewrite): Replace with a TypeParams type
    pub type_params: Option<NodeId>,
    // TODO(bumpalo_rewrite): Replace with a Params type
    pub params: NodeId,
    // TODO(bumpalo_rewrite): Replace with an InOutType type
    pub in_out_types: Option<NodeId>,
    pub block: &'a Block<'a>,
    pub env: bool,
    pub wrapped: bool,
}

// HACK(bumpalo_rewrite): not everything has to be pub
// TODO(bumpalo_rewrite): Finally uncomment this
#[derive(Debug, Clone)]
pub struct Let<'a> {
    pub span_start: usize,
    pub span_end: usize,
    // TODO(bumpalo_rewrite): Replace with a VarDecl type
    pub variable_name: NodeId,
    // TODO(bumpalo_rewrite): Replace with a Type type
    pub ty: Option<NodeId>,
    pub initializer: PipelineOrExprOrAssign<'a>,
    pub is_mutable: bool,
}

// HACK(bumpalo_rewrite): not everything has to be pub
// TODO(bumpalo_rewrite): Finally uncomment this
#[derive(Debug, Clone)]
pub struct While<'a> {
    pub span_start: usize,
    pub span_end: usize,
    pub condition: &'a ExprVariants<'a>,
    pub block: &'a Block<'a>,
}

// HACK(bumpalo_rewrite): not everything has to be pub
// TODO(bumpalo_rewrite): Finally uncomment this
#[derive(Debug, Clone)]
pub struct For<'a> {
    pub span_start: usize,
    pub span_end: usize,
    // TODO(bumpalo_rewrite): Replace with an VarDecl type
    pub variable: NodeId,
    pub range: ExprVariants<'a>,
    pub block: &'a Block<'a>,
}

// HACK(bumpalo_rewrite): not everything has to be pub
#[derive(Debug, Clone)]
pub struct Loop<'a> {
    pub span_start: usize,
    pub span_end: usize,
    pub block: &'a Block<'a>,
}

// HACK(bumpalo_rewrite): not everything has to be pub
#[derive(Debug, Clone)]
pub struct Return<'a> {
    pub span_start: usize,
    pub span_end: usize,
    pub ret_val: Option<&'a ExprVariants<'a>>,
}

// HACK(bumpalo_rewrite): not everything has to be pub
#[derive(Debug, Clone)]
pub struct Continue {
    pub span_start: usize,
    pub span_end: usize,
}

// HACK(bumpalo_rewrite): not everything has to be pub
#[derive(Debug, Clone)]
pub struct Break {
    pub span_start: usize,
    pub span_end: usize,
}

// HACK(bumpalo_rewrite): not everything has to be pub
#[derive(Debug, Clone)]
pub struct Alias {
    pub span_start: usize,
    pub span_end: usize,
    // TODO(bumpalo_rewrite): Change to VarDecl type
    pub new_name: NodeId,
    // TODO(bumpalo_rewrite): Change to VarRef type
    pub old_name: NodeId,
}

// HACK(bumpalo_rewrite): not everything has to be pub
#[derive(Debug, Clone)]
pub struct Extern {
    pub span_start: usize,
    pub span_end: usize,
    // TODO(bumpalo_rewrite): Change to VarDecl type
    pub name: NodeId,
    // TODO(bumpalo_rewrite): Change to Params type
    pub params: NodeId,
}

// Pipeline just contains a list of expressions
//
// It's not allowed if there is only one element in pipeline, in that
// case, it's just an expression.
//
// Making such restriction can reduce indirect access on expression, which
// can improve performance in parse time.
#[derive(Debug, Clone)]
pub struct Pipeline<'a> {
    pub span_start: usize,
    pub span_end: usize,
    // TODO(bumpalo_rewrite): Change to Vec of whatever
    pub nodes: Vec<&'a ExprVariants<'a>>,
}

// HACK(bumpalo_rewrite): not everything has to be pub
#[derive(Debug, Clone)]
// TODO(bumpalo_rewrite): This needs an enum inside it
// NOTE(bumpalo_rewrite): Possibly call it ExprVariants
// HACK(bumpalo_rewrite): Is this indirection needed??
pub struct Expr<'a> {
    pub span_start: usize,
    pub span_end: usize,
    // TODO(bumpalo_rewrite): This will be the ExprVariants defined below
    pub expression: ExprVariants<'a>,
}

// HACK(bumpalo_rewrite): not everything has to be pub
// NOTE(bumpalo_rewrite): This type and the Expr type both NEED to have the same type of
// the field 'expression'
#[derive(Debug, Clone)]
pub struct Assignment<'a> {
    pub span_start: usize,
    pub span_end: usize,
    // TODO(bumpalo_rewrite): This will be the ExprVariants defined below
    pub expression: ExprVariants<'a>,
}

// HACK(bumpalo_rewrite): not everything has to be pub
#[derive(Debug, Clone)]
pub enum PipelineOrExprOrAssign<'a> {
    Pipeline(&'a Pipeline<'a>),
    Expression(&'a ExprVariants<'a>),
    Assignment(&'a Assignment<'a>),
}

impl<'a> PipelineOrExprOrAssign<'a> {
    fn get_span_end(&self) -> usize {
        match self {
            PipelineOrExprOrAssign::Pipeline(pipeline) => pipeline.span_end,
            PipelineOrExprOrAssign::Expression(expr) => expr.get_span_end(),
            PipelineOrExprOrAssign::Assignment(assignment) => assignment.span_end,
        }
    }
}

// TODO(bumpalo_rewrite): Write an impl for getting span_end
#[derive(Debug, Clone)]
pub enum ExprVariants<'a> {
    Record(&'a Record<'a>),
    Closure(&'a Closure<'a>),
    If(&'a If<'a>),
    Match(&'a Match<'a>),
    Try(&'a Try<'a>),
    BinaryOp(&'a BinaryOp<'a>),
    List(&'a List),

    // TODO(bumpalo_rewrite): Should I keep
    // TODO(bumpalo_rewrite): Change to raw span struct and possibly use it everywhere
    // TODO(bumpalo_rewrite): There's already a Span struct, use that
    Garbage(usize, usize),
}

impl<'a> ExprVariants<'a> {
    fn get_span_start(&self) -> usize {
        match self {
            Self::Record(record) => record.span_start,
            Self::Closure(closure) => closure.span_start,
            Self::If(if_) => if_.span_end,
            Self::Match(match_) => match_.span_start,
            Self::Try(try_) => try_.span_start,
            Self::Garbage(span_start, _) => *span_start,
            Self::BinaryOp(binary_op) => binary_op.span_start,
            Self::List(list) => list.span_start,
        }
    }

    fn get_span_end(&self) -> usize {
        match self {
            Self::Record(record) => record.span_end,
            Self::Closure(closure) => closure.span_end,
            Self::If(if_) => if_.span_end,
            Self::Match(match_) => match_.span_end,
            Self::Try(try_) => try_.span_end,
            Self::Garbage(_, span_end) => *span_end,
            Self::BinaryOp(binary_op) => binary_op.span_end,
            Self::List(list) => list.span_end,
        }
    }
}

// HACK(bumpalo_rewrite): not everything has to be pub
#[derive(Debug, Clone)]
pub struct Record<'a> {
    pub span_start: usize,
    pub span_end: usize,
    // TODO(bumpalo_rewrite): Is this supposed to be like this, or more?
    pub pairs: Vec<(ExprVariants<'a>, ExprVariants<'a>)>,
}

// HACK(bumpalo_rewrite): not everything has to be pub
#[derive(Debug, Clone)]
pub struct Closure<'a> {
    pub span_start: usize,
    pub span_end: usize,
    // TODO(bumpalo_rewrite): Type this well
    pub params: Option<NodeId>,
    pub block: &'a Block<'a>,
}

// HACK(bumpalo_rewrite): not everything has to be pub
#[derive(Debug, Clone)]
pub struct If<'a> {
    pub span_start: usize,
    pub span_end: usize,
    pub condition: &'a ExprVariants<'a>,
    pub then_block: &'a Block<'a>,
    pub else_block: Option<&'a Block<'a>>,
}

// HACK(bumpalo_rewrite): not everything has to be pub
#[derive(Debug, Clone)]
pub struct Match<'a> {
    pub span_start: usize,
    pub span_end: usize,
    pub target: ExprVariants<'a>,
    pub match_arms: Vec<(ExprVariants<'a>, ExprVariants<'a>)>,
}

// HACK(bumpalo_rewrite): not everything has to be pub
#[derive(Debug, Clone)]
pub struct Try<'a> {
    pub span_start: usize,
    pub span_end: usize,
    pub try_block: &'a Block<'a>,
    pub catch_block: Option<&'a Block<'a>>,
    pub finally_block: Option<&'a Block<'a>>,
}

// HACK(bumpalo_rewrite): not everything has to be pub
#[derive(Debug, Clone)]
pub struct BinaryOp<'a> {
    pub span_start: usize,
    pub span_end: usize,
    // TODO(bumpalo_rewrite): Make it references instead??
    // NOTE(bumpalo_rewrite): Too much indirection imho
    pub lhs: ExprVariants<'a>,
    // TODO(bumpalo_rewrite): Make it a separate enum
    pub op: NodeId,
    pub rhs: PipelineOrExprOrAssign<'a>,
}

// HACK(bumpalo_rewrite): not everything has to be pub
#[derive(Debug, Clone)]
pub struct List {
    pub span_start: usize,
    pub span_end: usize,
    // TODO(bumpalo_rewrite): Figure out what type to place here
    pub items: Vec<NodeId>,
}

// HACK(bumpalo_rewrite): not everything has to be pub
#[derive(Debug, Clone)]
pub enum AssignOrExpr<'a> {
    Expression(&'a ExprVariants<'a>),
    Assignment(&'a Assignment<'a>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Params {
    pub nodes: Vec<NodeId>,
}

impl Params {
    pub fn new(nodes: Vec<NodeId>) -> Self {
        Self { nodes }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InOutTypes {
    pub nodes: Vec<NodeId>,
}

impl InOutTypes {
    pub fn new(nodes: Vec<NodeId>) -> Self {
        Self { nodes }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    pub parts: Vec<NodeId>,
}

impl Call {
    pub fn new(parts: Vec<NodeId>) -> Self {
        Self { parts }
    }
}

// #[derive(Debug, Clone, PartialEq)]
// pub struct List {
//     pub items: Vec<NodeId>,
// }

// impl List {
//     pub fn new(items: Vec<NodeId>) -> Self {
//         Self { items }
//     }
// }

#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub header: NodeId,
    pub rows: Vec<NodeId>,
}

impl Table {
    pub fn new(header: NodeId, rows: Vec<NodeId>) -> Self {
        Self { header, rows }
    }
}

// impl Record {
//     pub fn new(pairs: Vec<(NodeId, NodeId)>) -> Self {
//         Self { pairs }
//     }
// }

// impl Match {
//     pub fn new(target: NodeId, match_arms: Vec<(NodeId, NodeId)>) -> Self {
//         Self { target, match_arms }
//     }
// }

#[derive(Debug, Clone, PartialEq)]
pub struct TypeArgs {
    pub args: Vec<NodeId>,
}

impl TypeArgs {
    pub fn new(args: Vec<NodeId>) -> Self {
        Self { args }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockContext {
    /// This block is a whole block of code not wrapped in curlies (e.g., a file)
    Bare,
    /// This block is wrapped in curlies
    Curlies,
    /// This block should be parsed as part of a closure starting after closure params
    Closure,
}

#[derive(Debug)]
pub enum ParamsContext {
    /// Params for a command signature
    Squares,
    /// Params for a closure
    Pipes,
    /// Fields for a record
    Angles,
}

#[derive(Debug)]
pub enum BarewordContext {
    /// Bareword is a string (e.g., in a list)
    String,
    /// Bareword is a name (e.g., in a call position)
    Call,
}

// enum AssignmentOrExpression {
//     Assignment(NodeId),
//     Expression(NodeId),
// }

// impl AssignmentOrExpression {
//     fn get_node_id(&self) -> NodeId {
//         match self {
//             AssignmentOrExpression::Assignment(i) | AssignmentOrExpression::Expression(i) => *i,
//         }
//     }
// }

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum AstNode {
    Int,
    Float,
    String,
    Name,
    Type {
        name: NodeId,
        args: Option<NodeId>,
        optional: bool,
    },
    TypeArgs(TypeArgsId),
    RecordType {
        /// Contains [AstNode::Params]
        fields: NodeId,
        optional: bool,
    },
    Variable,

    // Booleans
    True,
    False,

    // Empty values
    Null,

    // Operators
    Pow,
    Multiply,
    Divide,
    FloorDiv,
    Modulo,
    Plus,
    Minus,
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
    RegexMatch,
    NotRegexMatch,
    In,
    Append,
    And,
    Xor,
    Or,

    // Assignments
    Assignment,
    AddAssignment,
    SubtractAssignment,
    MultiplyAssignment,
    DivideAssignment,
    AppendAssignment,

    // Statements
    Let {
        variable_name: NodeId,
        ty: Option<NodeId>,
        initializer: NodeId,
        is_mutable: bool,
    },
    While {
        condition: NodeId,
        block: NodeId,
    },
    For {
        variable: NodeId,
        range: NodeId,
        block: NodeId,
    },
    Loop {
        block: NodeId,
    },
    Return(Option<NodeId>),
    Break,
    Continue,

    // Definitions
    Def {
        name: NodeId,
        type_params: Option<NodeId>,
        params: NodeId,
        in_out_types: Option<NodeId>,
        block: NodeId,
        env: bool,
        wrapped: bool,
    },
    Extern {
        name: NodeId,
        params: NodeId,
    },
    Params(ParamsId),
    Param {
        name: NodeId,
        ty: Option<NodeId>,
    },
    InOutTypes(InOutTypesId),
    /// Input/output type pair for a command
    InOutType(NodeId, NodeId),
    Closure {
        params: Option<NodeId>,
        block: NodeId,
    },
    Alias {
        new_name: NodeId,
        old_name: NodeId,
    },

    /// Long flag ('--' + one or more letters)
    FlagLong,
    /// Short flag ('-' + single letter)
    FlagShort,
    /// Group of short flags ('-' + more than 1 letters)
    FlagShortGroup,

    // Expressions
    Call(CallId),
    NamedValue {
        name: NodeId,
        value: NodeId,
    },
    BinaryOp {
        lhs: NodeId,
        op: NodeId,
        rhs: NodeId,
    },
    Range {
        lhs: NodeId,
        rhs: NodeId,
    },
    List(ListId),
    Table(TableId),
    Record(RecordId),
    MemberAccess {
        target: NodeId,
        field: NodeId,
    },
    Block(BlockId),
    Pipeline(PipelineId),
    If {
        condition: NodeId,
        then_block: NodeId,
        else_block: Option<NodeId>,
    },
    Try {
        try_block: NodeId,
        catch_block: Option<NodeId>,
        finally_block: Option<NodeId>,
    },
    Match(MatchId),
    Statement(NodeId),
    Garbage,
}

pub const ASSIGNMENT_PRECEDENCE: usize = 10;

impl AstNode {
    pub fn precedence(&self) -> usize {
        match self {
            AstNode::Pow => 100,
            AstNode::Multiply | AstNode::Divide | AstNode::FloorDiv | AstNode::Modulo => 95,
            AstNode::Plus | AstNode::Minus => 90,
            AstNode::LessThan
            | AstNode::LessThanOrEqual
            | AstNode::GreaterThan
            | AstNode::GreaterThanOrEqual
            | AstNode::Equal
            | AstNode::NotEqual
            | AstNode::RegexMatch
            | AstNode::NotRegexMatch
            | AstNode::In
            | AstNode::Append => 80,
            AstNode::And => 50,
            AstNode::Xor => 45,
            AstNode::Or => 40,
            AstNode::Assignment
            | AstNode::AddAssignment
            | AstNode::SubtractAssignment
            | AstNode::MultiplyAssignment
            | AstNode::DivideAssignment
            | AstNode::AppendAssignment => ASSIGNMENT_PRECEDENCE,
            _ => 0,
        }
    }
}

impl Parser {
    pub fn new(compiler: Compiler, tokens: Tokens) -> Self {
        Self { compiler, tokens }
    }

    fn position(&mut self) -> usize {
        self.tokens.peek_span().start
    }

    // fn get_span_end(&self, node_id: NodeId) -> usize {
    //     self.compiler.spans[node_id.0].end
    // }

    pub fn parse<'a>(mut self, arena: &'a bumpalo::Bump) -> &'a Block<'a> {
        let _span = span!();
        // TODO(bumpalo_rewrite): Figure out the lifetime issue
        self.block(BlockContext::Bare, arena)
    }

    pub fn expression<'a>(&mut self, arena: &'a bumpalo::Bump) -> &'a ExprVariants<'a> {
        let _span = span!();
        match self.math_expression(false, arena) {
            AssignOrExpr::Expression(expr) => expr,
            AssignOrExpr::Assignment(assignment) => &assignment.expression,
        }
    }

    fn pipeline<'a>(
        &mut self,
        first_element: &'a ExprVariants<'a>,
        span_start: usize,
        arena: &'a bumpalo::Bump,
    ) -> &'a Pipeline<'a> {
        let mut expressions = vec![first_element];
        while self.is_pipe() {
            self.pipe();
            // maybe a new time
            if self.is_newline() {
                self.tokens.advance()
            }
            expressions.push(self.expression(arena));
        }
        let span_end = self.position();
        arena.alloc(Pipeline {
            span_start,
            span_end,
            nodes: expressions,
        })
    }
    pub fn pipeline_or_expression_or_assignment<'a>(
        &mut self,
        arena: &'a bumpalo::Bump,
    ) -> PipelineOrExprOrAssign<'a> {
        // get the first expression
        let _span = span!();
        let span_start = self.position();
        let first = self.math_expression(true, arena);

        match first {
            AssignOrExpr::Assignment(assignment) => PipelineOrExprOrAssign::Assignment(assignment),
            AssignOrExpr::Expression(expr) if !self.is_pipe() => {
                PipelineOrExprOrAssign::Expression(expr)
            }
            AssignOrExpr::Expression(expr) => {
                PipelineOrExprOrAssign::Pipeline(self.pipeline(expr, span_start, arena))
            }
        }
    }

    // TODO(bumpalo_rewrite): Make a PipelineOrExpression enum and add it there
    pub fn pipeline_or_expression<'a>(
        &mut self,
        arena: &'a bumpalo::Bump,
    ) -> PipelineOrExprOrAssign<'a> {
        let _span = span!();
        let span_start = self.position();
        let first = self.expression(arena);
        // pipeline with one element is an expression actually.
        if !self.is_pipe() {
            return PipelineOrExprOrAssign::Expression(first);
        }
        PipelineOrExprOrAssign::Pipeline(self.pipeline(first, span_start, arena))
    }

    fn math_expression<'a>(
        &mut self,
        allow_assignment: bool,
        arena: &'a bumpalo::Bump,
    ) -> AssignOrExpr<'a> {
        let _span = span!();
        // TODO(bumpalo_rewrite): The left expression in the tuple is supposed to be the op
        let mut expr_stack = Vec::<(NodeId, ExprVariants<'a>)>::new();

        let mut last_prec = 1000000;

        let span_start = self.position();

        // Check for special forms
        let expression = if self.is_keyword(b"if") {
            let if_ = self.if_expression(arena);
            Some(arena.alloc(ExprVariants::If(if_)))
        } else if self.is_keyword(b"match") {
            let match_ = self.match_expression(arena);
            Some(arena.alloc(ExprVariants::Match(match_)))
        } else if self.is_keyword(b"try") {
            let try_ = self.try_expression(arena);
            Some(arena.alloc(ExprVariants::Try(try_)))
        } else {
            None
        };

        if let Some(expression) = expression {
            return AssignOrExpr::Expression(expression);
        }
        // TODO
        // } else if self.is_keyword(b"where") {
        // }

        // Otherwise assume a math expression
        let mut leftmost = self.simple_expression(BarewordContext::Call, arena);

        if self.is_equals() {
            if !allow_assignment {
                self.error("assignment found in expression");
            }
            let op = self.operator();

            let rhs = self.pipeline_or_expression(arena);
            let span_end = rhs.get_span_end();

            let expression = arena.alloc(BinaryOp {
                span_start,
                span_end,
                lhs: leftmost,
                op,
                rhs,
            });
            return AssignOrExpr::Assignment(arena.alloc(Assignment {
                span_start,
                span_end,
                expression: ExprVariants::BinaryOp(expression),
            }));
        }

        while self.has_tokens() {
            if self.is_operator() {
                let missing_space_before_op = !self.is_horizontal_space();
                let op = self.operator();
                let missing_space_after_op = !self.is_horizontal_space();

                if missing_space_before_op {
                    self.error_on_node("missing space before operator", op);
                }

                if missing_space_after_op {
                    self.error_on_node("missing space after operator", op);
                }

                let op_prec = self.operator_precedence(op);

                if op_prec == ASSIGNMENT_PRECEDENCE && !allow_assignment {
                    self.error_on_node("assignment found in expression", op);
                }

                let rhs = if self.is_simple_expression() {
                    self.simple_expression(BarewordContext::Call, arena)
                } else {
                    self.error("incomplete math expression");
                    // TODO(bumpalo_rewrite): Ugh, span_end
                    ExprVariants::Garbage(span_start, self.tokens.peek().1.end)
                };

                while op_prec <= last_prec {
                    let Some((op, rhs)) = expr_stack.pop() else {
                        break;
                    };

                    last_prec = self.operator_precedence(op);

                    if last_prec < op_prec {
                        expr_stack.push((op, rhs));
                        break;
                    }

                    let lhs = expr_stack.last_mut().map_or(&mut leftmost, |l| &mut l.1);

                    let (span_start, span_end) = (lhs.get_span_start(), rhs.get_span_end());
                    let binary_op = arena.alloc(BinaryOp {
                        span_start,
                        span_end,
                        // TODO(bumpalo_rewrite): Ugh
                        lhs: lhs.clone(),
                        op,
                        rhs: PipelineOrExprOrAssign::Expression(arena.alloc(rhs)),
                    });
                    *lhs = ExprVariants::BinaryOp(binary_op);
                }

                expr_stack.push((op, rhs));

                last_prec = op_prec;
            } else {
                break;
            }
        }

        while let Some((op, rhs)) = expr_stack.pop() {
            let lhs = expr_stack.last_mut().map_or(&mut leftmost, |l| &mut l.1);

            let (span_start, span_end) = (lhs.get_span_start(), rhs.get_span_end());

            let binary_op = arena.alloc(BinaryOp {
                span_start,
                span_end,
                // TODO(bumpalo_rewrite): Ugh..
                lhs: lhs.clone(),
                op,
                rhs: PipelineOrExprOrAssign::Expression(arena.alloc(rhs)),
            });
            *lhs = ExprVariants::BinaryOp(binary_op);
        }

        AssignOrExpr::Expression(arena.alloc(leftmost))
    }

    pub fn simple_expression<'a>(
        &mut self,
        bareword_context: BarewordContext,
        arena: &'a bumpalo::Bump,
    ) -> ExprVariants<'a> {
        let _span = span!();

        // skip comments and newlines
        while self.is_comment() || self.is_newline() {
            self.tokens.advance();
        }

        let span_start = self.position();

        let (token, span) = self.tokens.peek();

        let mut expr = match token {
            Token::LCurly => self.record_or_closure(arena),
            Token::LParen => {
                self.tokens.advance();
                if self.tokens.peek_token() == Token::RParen {
                    self.error("use null instead of ()");
                    // TODO(bumpalo_rewrite): Is it supposed to be span_start + 1 or span_start + 2??
                    ExprVariants::Garbage(span_start, span_start + 1)
                } else {
                    let output = self.expression(arena);
                    self.rparen();
                    // TODO(bumpalo_rewrite): Ugh
                    output.clone()
                }
            }
            // TODO(bumpalo_rewrite): Fix this function, RESUME FROM HERE AFTER THIS FUNCTION CALL IS FIXED!!!
            Token::LSquare => self.list_or_table(arena),
            Token::Int => self.advance_node(AstNode::Int, span),
            Token::Float => self.advance_node(AstNode::Float, span),
            Token::DoubleQuotedString => self.advance_node(AstNode::String, span),
            Token::SingleQuotedString => self.advance_node(AstNode::String, span),
            Token::Dollar => self.variable(),
            Token::Bareword => match self.compiler.get_span_contents_manual(span.start, span.end) {
                b"true" => self.advance_node(AstNode::True, span),
                b"false" => self.advance_node(AstNode::False, span),
                b"null" => self.advance_node(AstNode::Null, span),
                _ => match bareword_context {
                    BarewordContext::String => {
                        let node_id = self.name();
                        self.compiler.ast_nodes[node_id.0] = AstNode::String;
                        node_id
                    }
                    BarewordContext::Call => self.call(),
                },
            },
            _ => self.error("incomplete expression"),
        };

        loop {
            if self.is_horizontal_space() {
                return expr;
            } else if self.is_dotdot() {
                // Range
                self.tokens.advance();

                if self.is_horizontal_space() {
                    // TODO: implement range from
                    //
                    // TODO: tweak the garbage location.
                    self.error("incomplete range");
                    return expr;
                } else {
                    let rhs = self.simple_expression(BarewordContext::String, arena);
                    let span_end = rhs.get_span_end();

                    expr =
                        self.create_node(AstNode::Range { lhs: expr, rhs }, span_start, span_end);
                }
            } else if self.is_dot() {
                // Member access
                self.tokens.advance();

                if self.is_horizontal_space() {
                    self.error("missing path name");
                    return expr;
                }

                let name = self.name();

                let field_or_call = if self.is_lparen() {
                    self.variable()
                } else {
                    name
                };
                let span_end = self.get_span_end(field_or_call);

                match self.compiler.get_node_mut(field_or_call) {
                    AstNode::Variable | AstNode::Name => {
                        expr = self.create_node(
                            AstNode::MemberAccess {
                                target: expr,
                                field: field_or_call,
                            },
                            span_start,
                            span_end,
                        );
                    }
                    _ => {
                        self.error("expected field");
                    }
                }
            } else {
                return expr;
            }
        }
    }

    pub fn advance_node(&mut self, node: AstNode, span: Span) -> NodeId {
        self.tokens.advance();
        self.create_node(node, span.start, span.end)
    }

    pub fn variable(&mut self) -> NodeId {
        if self.is_dollar() {
            let span_start = self.position();
            self.tokens.advance();

            if let (Token::Bareword, name_span) = self.tokens.peek() {
                self.tokens.advance();
                self.create_node(AstNode::Variable, span_start, name_span.end)
            } else {
                self.error("variable name must be a bareword")
            }
        } else {
            self.error("expected variable starting with '$'")
        }
    }

    // TODO(bumpalo_rewrite): Make this a VarDecl type
    pub fn variable_decl(&mut self) -> NodeId {
        let _span = span!();

        let span_start = self.position();

        if self.is_dollar() {
            self.tokens.advance();
        }

        if let (Token::Bareword, name_span) = self.tokens.peek() {
            self.tokens.advance();
            self.create_node(AstNode::Variable, span_start, name_span.end)
        } else {
            self.error("variable assignment name must be a bareword")
        }
    }

    pub fn call<'a>(&mut self, arena: &'a bumpalo::Bump) -> NodeId {
        let _span = span!();
        let mut parts = vec![self.call_name()];
        let mut is_head = true;
        let span_start = self.position();

        while self.has_tokens() {
            if self.is_newline() {
                break;
            }

            if self.is_name() && is_head {
                parts.push(self.name());
                continue;
            }

            // TODO: Add flags

            is_head = false;
            let arg_id = self.simple_expression(BarewordContext::String, arena);
            parts.push(arg_id);
        }

        let span_end = self.position();

        self.compiler.calls.push(Call::new(parts));
        self.create_node(
            AstNode::Call(CallId(self.compiler.calls.len() - 1)),
            span_start,
            span_end,
        )
    }

    // TODO(bumpalo_rewrite): RESUME FROM HERE!!!!!!!!
    // TODO(bumpalo_rewrite): Add the variant to ExprVariants first
    pub fn list_or_table<'a>(&mut self, arena: &'a bumpalo::Bump) -> ExprVariants<'a> {
        let _span = span!();
        let span_start = self.position();
        let mut is_table = false;
        let mut items = vec![];

        self.lsquare();
        let mut span_end = self.position();

        loop {
            if self.is_rsquare() {
                span_end = self.position();
                self.tokens.advance();
                break;
            } else if self.is_comma() || self.is_newline() {
                // TODO: should we disallow `[,,,]`?
                self.tokens.advance();
            } else if self.is_semicolon() {
                if items.len() != 1 {
                    self.error("semicolon to create table should immediately follow headers");
                    let span_end = self.tokens.peek_span().end;
                    return ExprVariants::Garbage(span_start, span_end);
                } else if
                // TODO(bumpalo_rewrite): Fix this check please, once the appropriate variants are defined
                // in the enum
                !matches!(items[0], ExprVariants::List(_)) {
                    self.error("tables require a list for their headers");
                    let span_end = self.tokens.peek_span().end;
                    return ExprVariants::Garbage(span_start, span_end);
                }
                self.tokens.advance();
                is_table = true;
            } else if self.is_simple_expression() {
                items.push(self.simple_expression(BarewordContext::String, arena));
            } else {
                self.error("expected list item");
                items.push(ExprVariants::Garbage(span_start, span_end));
                if self.is_eof() {
                    // prevent forever looping if there is no token to put the error on
                    break;
                }
            }
        }

        if is_table {
            let header = items.remove(0);
            self.compiler.tables.push(Table::new(header, items));
            self.create_node(
                AstNode::Table(TableId(self.compiler.tables.len() - 1)),
                span_start,
                span_end,
            )
        } else {
            self.compiler.lists.push(List::new(items));
            self.create_node(
                AstNode::List(ListId(self.compiler.lists.len() - 1)),
                span_start,
                span_end,
            )
        }
    }

    pub fn record_or_closure<'a>(&mut self, arena: &'a bumpalo::Bump) -> ExprVariants<'a> {
        let _span = span!();
        let span_start = self.position();
        let mut span_end = self.position(); // TODO: make sure we only initialize it expectedly

        let mut is_closure = false;
        let mut first_pass = true;
        // For the record
        let mut items = vec![];

        self.lcurly();
        self.skip_newlines();

        // Explicit closure case
        if self.is_pipe() {
            // TODO(bumpalo_rewrite): Fix the params type
            let params = Some(self.signature_params(ParamsContext::Pipes));
            let block = self.block(BlockContext::Closure, arena);
            self.rcurly();
            span_end = self.position();

            let closure = arena.alloc(Closure {
                span_start,
                span_end,
                params,
                block,
            });
            return ExprVariants::Closure(closure);
        }

        let rollback_point = self.get_rollback_point();
        loop {
            self.skip_newlines();
            if self.is_rcurly() {
                self.rcurly();
                span_end = self.position();
                break;
            }
            let key = self.simple_expression(BarewordContext::String, arena);
            self.skip_newlines();
            if first_pass && !self.is_colon() {
                is_closure = true;
                break;
            }
            self.colon();
            self.skip_newlines();
            let val = self.simple_expression(BarewordContext::String, arena);
            items.push((key, val));
            first_pass = false;

            if self.is_comma() {
                self.comma()
            }
            if self.is_eof() {
                // abort when appropriate
                break;
            }
        }

        if is_closure {
            self.apply_rollback(rollback_point);
            let block = self.block(BlockContext::Closure, arena);
            self.rcurly();

            span_end = self.position();

            let closure = arena.alloc(Closure {
                span_start,
                span_end,
                params: None,
                block,
            });
            ExprVariants::Closure(closure)
        } else {
            let record = arena.alloc(Record {
                span_start,
                span_end,
                pairs: items,
            });
            ExprVariants::Record(record)
        }
    }

    // TODO(bumpalo_rewrite): make a separate enum and use that
    pub fn operator(&mut self) -> NodeId {
        let (token, span) = self.tokens.peek();

        match token {
            Token::Plus => self.advance_node(AstNode::Plus, span),
            Token::PlusPlus => self.advance_node(AstNode::Append, span),
            Token::Dash => self.advance_node(AstNode::Minus, span),
            Token::Asterisk => self.advance_node(AstNode::Multiply, span),
            Token::ForwardSlash => self.advance_node(AstNode::Divide, span),
            Token::ForwardSlashForwardSlash => self.advance_node(AstNode::FloorDiv, span),
            Token::LessThan => self.advance_node(AstNode::LessThan, span),
            Token::LessThanEqual => self.advance_node(AstNode::LessThanOrEqual, span),
            Token::GreaterThan => self.advance_node(AstNode::GreaterThan, span),
            Token::GreaterThanEqual => self.advance_node(AstNode::GreaterThanOrEqual, span),
            Token::EqualsEquals => self.advance_node(AstNode::Equal, span),
            Token::ExclamationEquals => self.advance_node(AstNode::NotEqual, span),
            Token::EqualsTilde => self.advance_node(AstNode::RegexMatch, span),
            Token::ExclamationTilde => self.advance_node(AstNode::NotRegexMatch, span),
            Token::AsteriskAsterisk => self.advance_node(AstNode::Pow, span),
            Token::Equals => self.advance_node(AstNode::Assignment, span),
            Token::PlusEquals => self.advance_node(AstNode::AddAssignment, span),
            Token::DashEquals => self.advance_node(AstNode::SubtractAssignment, span),
            Token::AsteriskEquals => self.advance_node(AstNode::MultiplyAssignment, span),
            Token::ForwardSlashEquals => self.advance_node(AstNode::DivideAssignment, span),
            Token::PlusPlusEquals => self.advance_node(AstNode::AppendAssignment, span),
            Token::Bareword => match self.compiler.get_span_contents_manual(span.start, span.end) {
                b"mod" => self.advance_node(AstNode::Modulo, span),
                b"in" => self.advance_node(AstNode::In, span),
                b"and" => self.advance_node(AstNode::And, span),
                b"xor" => self.advance_node(AstNode::Xor, span),
                b"or" => self.advance_node(AstNode::Or, span),
                op => self.error(format!(
                    "Unknown operator: '{}'",
                    String::from_utf8_lossy(op)
                )),
            },
            _ => self.error("expected: operator"),
        }
    }

    // TODO(bumpalo_rewrite): Change to use Operators enum and use that for precedence specifically
    pub fn operator_precedence(&mut self, operator: NodeId) -> usize {
        self.compiler.get_node(operator).precedence()
    }

    pub fn spanning(&mut self, from: NodeId, to: NodeId) -> (usize, usize) {
        (
            self.compiler.spans[from.0].start,
            self.compiler.spans[to.0].end,
        )
    }

    pub fn string(&mut self) -> NodeId {
        match self.tokens.peek() {
            (Token::DoubleQuotedString, span) => self.advance_node(AstNode::String, span),
            (Token::SingleQuotedString, span) => self.advance_node(AstNode::String, span),
            _ => self.error("expected: string"),
        }
    }

    pub fn name(&mut self) -> NodeId {
        match self.tokens.peek() {
            (Token::Bareword, span) => self.advance_node(AstNode::Name, span),
            _ => self.error("expected: name"),
        }
    }

    pub fn call_name(&mut self) -> NodeId {
        let (mut token, mut span) = self.tokens.peek();

        loop {
            if [Token::Eof, Token::Newline].contains(&token) {
                break;
            }

            self.tokens.advance();
            let (next_token, next_span) = self.tokens.peek();

            if next_span.start > span.end {
                // horizontal whitespace
                break;
            }

            token = next_token;
            span.end = next_span.end;
        }

        self.create_node(AstNode::Name, span.start, span.end)
    }

    pub fn has_tokens(&mut self) -> bool {
        self.tokens.peek_token() != Token::Eof
    }

    // TODO(bumpalo_rewrite): Add parametric lifetime when needed
    pub fn match_expression<'a>(&mut self, arena: &'a bumpalo::Bump) -> &'a Match<'a> {
        let _span = span!();
        let span_start = self.position();

        self.keyword(b"match");

        let target = self.simple_expression(BarewordContext::String, arena);
        let mut span_end = target.get_span_end().unwrap_or(span_start + b"match".len());

        let mut match_arms = vec![];

        if !self.is_lcurly() {
            self.error("expected left curly brace '{'");
            return arena.alloc(Match {
                span_start,
                span_end,
                target,
                match_arms: vec![],
            });
        }

        self.lcurly();

        loop {
            if self.is_rcurly() {
                span_end = self.position() + 1;
                self.rcurly();
                break;
            } else if self.is_simple_expression() {
                let pattern = self.simple_expression(BarewordContext::String, arena);
                let span_end = pattern.get_span_end().unwrap_or(span_end);

                if !self.is_thick_arrow() {
                    self.error("expected thick arrow (=>) between match cases");

                    return arena.alloc(Match {
                        span_start,
                        span_end,
                        target,
                        match_arms: vec![],
                    });
                }
                self.tokens.advance();

                let pattern_result = self.simple_expression(BarewordContext::String, arena);

                if self.is_comma() {
                    self.tokens.advance();
                }

                match_arms.push((pattern, pattern_result));
            } else if self.is_newline() {
                self.tokens.advance();
            } else {
                self.error("expected match arm in match");

                return arena.alloc(Match {
                    span_start,
                    span_end,
                    target,
                    match_arms: vec![],
                });
            }
        }

        arena.alloc(Match {
            span_start,
            span_end,
            target,
            match_arms,
        })
    }

    // TODO(bumpalo_rewrite): Do not use Option type here
    pub fn if_expression<'a>(&mut self, arena: &'a bumpalo::Bump) -> &'a If<'a> {
        let _span = span!();
        let span_start = self.position();
        let span_end;

        self.keyword(b"if");

        let condition = self.expression(arena);
        self.skip_newlines();

        let then_block = self.block(BlockContext::Curlies, arena);
        self.skip_newlines();

        let else_block: Option<&'_ Block<'_>> = if self.is_keyword(b"else") {
            self.tokens.advance();
            self.skip_newlines();

            let block = if self.is_keyword(b"if") {
                let if_ = self.if_expression(arena);
                let inner_span_start = if_.span_start;
                let inner_span_end = if_.span_end;
                let if_expr = arena.alloc(ExprVariants::If(if_));

                let expr = PipelineOrExprOrAssign::Expression(if_expr);
                arena.alloc(Block {
                    span_start: inner_span_start,
                    span_end: inner_span_end,
                    nodes: vec![BlockEntities::PipelineOrExprOrAssign(expr)],
                })
            } else if self.is_keyword(b"match") {
                let match_ = self.match_expression(arena);
                let inner_span_start = match_.span_start;
                let inner_span_end = match_.span_end;
                let match_expr = arena.alloc(ExprVariants::Match(match_));

                let expr = PipelineOrExprOrAssign::Expression(match_expr);
                arena.alloc(Block {
                    span_start: inner_span_start,
                    span_end: inner_span_end,
                    nodes: vec![BlockEntities::PipelineOrExprOrAssign(expr)],
                })
            } else {
                self.block(BlockContext::Curlies, arena)
            };

            span_end = block.span_end;
            Some(block)
        } else {
            span_end = then_block.span_end;
            None
        };

        arena.alloc(If {
            span_start,
            span_end,
            condition,
            then_block,
            else_block,
        })
    }

    pub fn try_expression<'a>(&mut self, arena: &'a bumpalo::Bump) -> &'a Try<'a> {
        let _span = span!();
        let span_start = self.position();

        self.keyword(b"try");

        let try_block = self.block(BlockContext::Curlies, arena);
        let mut span_end = try_block.span_end;
        self.skip_newlines();

        // catch
        let catch_block = if self.is_keyword(b"catch") {
            self.tokens.advance();
            self.skip_newlines();

            let block = self.block(BlockContext::Curlies, arena);
            span_end = block.span_end;

            Some(block)
        } else {
            None
        };

        // finally
        let finally_block = if self.is_keyword(b"finally") {
            self.tokens.advance();
            self.skip_newlines();

            let block = self.block(BlockContext::Curlies, arena);
            span_end = block.span_end;
            Some(block)
        } else {
            None
        };

        arena.alloc(Try {
            span_start,
            span_end,
            try_block,
            catch_block,
            finally_block,
        })
    }

    // directly ripped from `type_params` just changed delimiters
    // FIXME: simplify if appropriate
    pub fn signature_params(&mut self, params_context: ParamsContext) -> NodeId {
        let _span = span!();
        let span_start = self.position();
        let span_end;
        let param_list = {
            match params_context {
                ParamsContext::Pipes => self.pipe(),
                ParamsContext::Squares => self.lsquare(),
                ParamsContext::Angles => self.less_than(),
            }

            let mut output = vec![];

            while self.has_tokens() {
                match params_context {
                    ParamsContext::Pipes => {
                        if self.is_pipe() {
                            break;
                        }
                    }
                    ParamsContext::Squares => {
                        if self.is_rsquare() {
                            break;
                        }
                    }
                    ParamsContext::Angles => {
                        if self.is_greater_than() {
                            break;
                        }
                    }
                }

                if self.is_comma() {
                    self.tokens.advance();
                    continue;
                }

                let name = self.name();

                let ty = if self.is_colon() {
                    // We have a type
                    self.colon();

                    Some(self.typename())
                } else {
                    None
                };

                let name_span = self.compiler.spans[name.0];
                let param_span_end = if let Some(ty_id) = ty {
                    self.compiler.spans[ty_id.0].end
                } else {
                    name_span.end
                };

                let param =
                    self.create_node(AstNode::Param { name, ty }, name_span.start, param_span_end);

                // output.push(self.name());
                output.push(param);
            }

            span_end = self.position() + 1;

            match params_context {
                ParamsContext::Pipes => self.pipe(),
                ParamsContext::Squares => self.rsquare(),
                ParamsContext::Angles => self.greater_than(),
            }

            output
        };

        self.compiler.params.push(Params::new(param_list));
        self.create_node(
            AstNode::Params(ParamsId(self.compiler.params.len() - 1)),
            span_start,
            span_end,
        )
    }

    pub fn type_params(&mut self) -> NodeId {
        let _span = span!();
        let span_start = self.position();
        self.less_than();

        let mut param_list = vec![];

        while self.has_tokens() {
            if self.is_greater_than() {
                break;
            }

            if self.is_comma() {
                self.tokens.advance();
                continue;
            }

            param_list.push(self.name());
        }

        let span_end = self.position() + 1;
        self.greater_than();

        self.compiler.params.push(Params::new(param_list));
        self.create_node(
            AstNode::Params(ParamsId(self.compiler.params.len() - 1)),
            span_start,
            span_end,
        )
    }

    pub fn type_args(&mut self) -> NodeId {
        let _span = span!();
        let span_start = self.position();
        let span_end;
        let arg_list = {
            self.less_than();

            let mut output = vec![];

            while self.has_tokens() {
                if self.is_greater_than() {
                    break;
                }

                if self.is_comma() {
                    self.tokens.advance();
                    continue;
                }

                output.push(self.typename());
            }

            span_end = self.position() + 1;
            self.greater_than();

            output
        };

        self.compiler.type_args.push(TypeArgs::new(arg_list));
        self.create_node(
            AstNode::TypeArgs(TypeArgsId(self.compiler.type_args.len() - 1)),
            span_start,
            span_end,
        )
    }

    pub fn typename(&mut self) -> NodeId {
        let _span = span!();
        if let (Token::Bareword, span) = self.tokens.peek() {
            let name = self.name();
            let name_text = self.compiler.get_span_contents(name);

            if name_text == b"record" {
                let fields = self.signature_params(ParamsContext::Angles);
                let optional = if self.is_question_mark() {
                    // We have an optional type
                    self.tokens.advance();
                    true
                } else {
                    false
                };
                let span_end = self.position();
                return self.create_node(
                    AstNode::RecordType { fields, optional },
                    span.start,
                    span_end,
                );
            }

            let mut args = None;
            if self.is_less_than() {
                // We have generics
                args = Some(self.type_args());
            }

            let optional = if self.is_question_mark() {
                // We have an optional type
                self.tokens.advance();
                true
            } else {
                false
            };
            self.create_node(
                AstNode::Type {
                    name,
                    args,
                    optional,
                },
                span.start,
                span.end, // FIXME: this uses the end of the name as its end
            )
        } else {
            self.error("expect name")
        }
    }

    pub fn in_out_type(&mut self) -> NodeId {
        let _span = span!();
        let span_start = self.position();

        let in_ty = self.typename();
        self.thin_arrow();
        let out_ty = self.typename();

        let span_end = self.position();
        self.create_node(AstNode::InOutType(in_ty, out_ty), span_start, span_end)
    }

    pub fn in_out_types(&mut self) -> NodeId {
        let _span = span!();
        self.colon();

        if self.is_lsquare() {
            let span_start = self.position();

            self.tokens.advance();

            let mut output = vec![];
            while self.has_tokens() {
                if self.is_rsquare() {
                    break;
                }

                if self.is_comma() {
                    self.tokens.advance();
                    continue;
                }

                output.push(self.in_out_type());
            }

            self.rsquare();
            let span_end = self.position();

            self.compiler.in_out_types.push(InOutTypes::new(output));
            self.create_node(
                AstNode::InOutTypes(InOutTypesId(self.compiler.in_out_types.len() - 1)),
                span_start,
                span_end,
            )
        } else {
            let ty = self.in_out_type();
            let span = self.compiler.get_span(ty);
            self.compiler.in_out_types.push(InOutTypes::new(vec![ty]));
            self.create_node(
                AstNode::InOutTypes(InOutTypesId(self.compiler.in_out_types.len() - 1)),
                span.start,
                span.end,
            )
        }
    }

    pub fn def_statement<'a>(&mut self, arena: &'a bumpalo::Bump) -> Option<&'a Def<'a>> {
        let _span = span!();
        let span_start = self.position();

        self.keyword(b"def");
        let mut has_env_flag = false;
        let mut has_wrapped_flag = false;

        // maybe `--env` or `--wrapped`
        while let (Token::DashDash, _) = self.tokens.peek() {
            self.tokens.advance();
            match self.tokens.peek() {
                // let's make sure that the word is `env` or `wrapped`
                (Token::Bareword, span) => {
                    let flag_name = self.compiler.get_span_contents_manual(span.start, span.end);
                    if flag_name == b"env" {
                        if has_env_flag {
                            // TODO(bumpalo_rewrite): Figure out how to deal with the errors
                            self.error("duplicated --env flag");
                            return None;
                        }
                        has_env_flag = true;
                    } else if flag_name == b"wrapped" {
                        if has_wrapped_flag {
                            self.error("duplicated --wrapped flag");
                            return None;
                        }
                        has_wrapped_flag = true
                    } else {
                        self.error("expect --env or --wrapped");
                        return None;
                    }
                    self.tokens.advance();
                }
                _ => {
                    self.error("incomplete flag name");
                    return None;
                }
            }
        }

        let name = match self.tokens.peek() {
            (Token::Bareword, span) => self.advance_node(AstNode::Name, span),
            (Token::DoubleQuotedString | Token::SingleQuotedString, span) => {
                // TODO(bumpalo_rewrite): Create a specific VarDecl node
                self.advance_node(AstNode::String, span)
            }
            _ => {
                self.error("expected def name");
                return None;
            }
        };

        let type_params = self.is_less_than().then(|| self.type_params());

        let params = self.signature_params(ParamsContext::Squares);
        let in_out_types = if self.is_colon() {
            Some(self.in_out_types())
        } else {
            None
        };
        let block = self.block(BlockContext::Curlies, arena);

        let span_end = block.span_end;

        let def = arena.alloc(Def {
            span_start,
            span_end,
            name,
            type_params,
            params,
            in_out_types,
            block,
            env: has_env_flag,
            wrapped: has_wrapped_flag,
        });

        Some(def)
    }

    pub fn extern_statement<'a>(&mut self, arena: &'a bumpalo::Bump) -> Option<&'a Extern> {
        let _span = span!();
        let span_start = self.position();

        self.keyword(b"extern");

        // TODO(bumpalo_rewrite): Do it in VarDecl scope
        let name = match self.tokens.peek() {
            (Token::Bareword, span) => self.advance_node(AstNode::Name, span),
            (Token::DoubleQuotedString | Token::SingleQuotedString, span) => {
                self.advance_node(AstNode::String, span)
            }
            _ => {
                self.error("expected def name");
                return None;
            }
        };

        let params = self.signature_params(ParamsContext::Squares);
        let span_end = self.position();

        Some(arena.alloc(Extern {
            span_start,
            span_end,
            name,
            params,
        }))
    }

    // TODO: Deduplicate code between let/mut/const assignments
    pub fn let_statement<'a>(&mut self, arena: &'a bumpalo::Bump) -> &'a Let<'a> {
        let _span = span!();
        let is_mutable = false;
        let span_start = self.position();

        self.keyword(b"let");

        // TODO(bumpalo_rewrite): This becomes a reference to VarDecl type
        let variable_name = self.variable_decl();

        let ty = self.is_colon().then(|| {
            // We have a type
            self.colon();
            self.typename()
        });

        self.equals();

        let initializer = self.pipeline_or_expression(arena);

        let span_end = match initializer {
            PipelineOrExprOrAssign::Pipeline(pipeline) => pipeline.span_end,
            PipelineOrExprOrAssign::Expression(expr) => expr.get_span_end(),
            PipelineOrExprOrAssign::Assignment(assignment) => assignment.span_end,
        };

        arena.alloc(Let {
            span_start,
            span_end,
            variable_name,
            ty,
            initializer,
            is_mutable,
        })
    }

    // TODO: Deduplicate code between let/mut/const assignments
    pub fn mut_statement<'a>(&mut self, arena: &'a bumpalo::Bump) -> &'a Let<'a> {
        let _span = span!();
        let is_mutable = true;
        let span_start = self.position();

        self.keyword(b"mut");

        let variable_name = self.variable_decl();

        let ty = if self.is_colon() {
            // We have a type
            self.colon();

            Some(self.typename())
        } else {
            None
        };

        self.equals();

        let initializer = self.pipeline_or_expression(arena);

        let span_end = initializer.get_span_end();

        arena.alloc(Let {
            span_start,
            span_end,
            variable_name,
            ty,
            initializer,
            is_mutable,
        })
    }

    pub fn keyword(&mut self, keyword: &[u8]) {
        let _span = span!();
        if self.is_keyword(keyword) {
            self.tokens.advance();
        } else {
            self.error(format!(
                "expected keyword: {}",
                String::from_utf8_lossy(keyword)
            ));
        }
    }

    pub fn block<'a>(&mut self, context: BlockContext, arena: &'a bumpalo::Bump) -> &'a Block<'a> {
        let _span = span!();
        let span_start = self.position();

        let mut code_body = vec![];
        if let BlockContext::Curlies = context {
            self.lcurly();
        }

        while self.has_tokens() {
            if self.is_rcurly() && context == BlockContext::Curlies {
                self.rcurly();
                break;
            } else if self.is_rcurly() && context == BlockContext::Closure {
                // not responsible for parsing it, yield back to the closure pass
                break;
            } else if self.is_semicolon() || self.is_newline() || self.is_comment() {
                self.tokens.advance();
                continue;
            } else if self.is_keyword(b"def") {
                match self.def_statement(arena) {
                    Some(def) => {
                        code_body.push(BlockEntities::Def(def));
                    }
                    None => {}
                };
            } else if self.is_keyword(b"let") {
                code_body.push(BlockEntities::Let(self.let_statement(arena)));
            } else if self.is_keyword(b"mut") {
                code_body.push(BlockEntities::Let(self.mut_statement(arena)));
            } else if self.is_keyword(b"while") {
                match self.while_statement(arena) {
                    Some(while_) => {
                        code_body.push(BlockEntities::While(while_));
                    }
                    None => {}
                };
            } else if self.is_keyword(b"for") {
                code_body.push(BlockEntities::For(self.for_statement(arena)));
            } else if self.is_keyword(b"loop") {
                code_body.push(BlockEntities::Loop(self.loop_statement(arena)));
            } else if self.is_keyword(b"return") {
                code_body.push(BlockEntities::Return(self.return_statement(arena)));
            } else if self.is_keyword(b"continue") {
                code_body.push(BlockEntities::Continue(self.continue_statement(arena)));
            } else if self.is_keyword(b"break") {
                code_body.push(BlockEntities::Break(self.break_statement(arena)));
            } else if self.is_keyword(b"alias") {
                code_body.push(BlockEntities::Alias(self.alias_statement(arena)));
            } else if self.is_keyword(b"extern") {
                match self.extern_statement(arena) {
                    Some(extern_) => {
                        code_body.push(BlockEntities::Extern(extern_));
                    }
                    None => {}
                }
            } else {
                let pipeline = self.pipeline_or_expression_or_assignment(arena);

                if self.is_semicolon() {
                    // This is a statement, not an expression
                    self.tokens.advance();
                    code_body.push(BlockEntities::Statement(pipeline));
                } else {
                    code_body.push(BlockEntities::PipelineOrExprOrAssign(pipeline));
                }
            }
        }

        let span_end = self.position();

        arena.alloc(Block {
            span_start,
            span_end,
            nodes: code_body,
        })
    }

    pub fn while_statement<'a>(&mut self, arena: &'a bumpalo::Bump) -> Option<&'a While<'a>> {
        let _span = span!();
        let span_start = self.position();
        self.keyword(b"while");

        if self.is_operator() {
            // TODO: flag parsing
            self.error("WIP: Flags on while are not supported yet");
            self.tokens.advance();
            return None;
        }

        let condition = self.expression(arena);
        let block = self.block(BlockContext::Curlies, arena);
        let span_end = block.span_end;

        Some(arena.alloc(While {
            span_start,
            span_end,
            condition,
            block,
        }))
    }

    pub fn for_statement<'a>(&mut self, arena: &'a bumpalo::Bump) -> &'a For<'a> {
        let _span = span!();
        let span_start = self.position();
        self.keyword(b"for");

        let variable = self.variable_decl();
        self.keyword(b"in");

        let range = self.simple_expression(BarewordContext::String, arena);
        let block = self.block(BlockContext::Curlies, arena);
        let span_end = block.span_end;

        arena.alloc(For {
            span_start,
            span_end,
            variable,
            range,
            block,
        })
    }

    pub fn loop_statement<'a>(&mut self, arena: &'a bumpalo::Bump) -> &'a Loop<'a> {
        let _span = span!();
        let span_start = self.position();
        self.keyword(b"loop");
        let block = self.block(BlockContext::Curlies, arena);
        let span_end = block.span_end;

        arena.alloc(Loop {
            span_start,
            span_end,
            block,
        })
    }

    pub fn return_statement<'a>(&mut self, arena: &'a bumpalo::Bump) -> &'a Return<'a> {
        let _span = span!();
        let span_start = self.position();
        let span_end;

        self.keyword(b"return");

        let ret_val = if self.is_expression() {
            let expr = self.expression(arena);
            span_end = expr.get_span_end();
            Some(expr)
        } else {
            span_end = span_start + b"return".len();
            None
        };

        arena.alloc(Return {
            span_start,
            span_end,
            ret_val,
        })
        // self.create_node(AstNode::Return(ret_val), span_start, span_end)
    }

    pub fn continue_statement<'a>(&mut self, arena: &'a bumpalo::Bump) -> &'a Continue {
        let _span = span!();
        let span_start = self.position();
        self.keyword(b"continue");
        let span_end = span_start + b"continue".len();

        arena.alloc(Continue {
            span_start,
            span_end,
        })
        // self.create_node(AstNode::Continue, span_start, span_end)
    }

    pub fn break_statement<'a>(&mut self, arena: &'a bumpalo::Bump) -> &'a Break {
        let _span = span!();
        let span_start = self.position();
        self.keyword(b"break");
        let span_end = span_start + b"break".len();

        arena.alloc(Break {
            span_start,
            span_end,
        })
        // self.create_node(AstNode::Break, span_start, span_end)
    }

    pub fn alias_statement<'a>(&mut self, arena: &'a bumpalo::Bump) -> &'a Alias {
        let _span = span!();
        let span_start = self.position();
        self.keyword(b"alias");
        let new_name = if self.is_string() {
            self.string()
        } else {
            self.name()
        };
        self.equals();
        let old_name = if self.is_string() {
            self.string()
        } else {
            self.name()
        };
        // TODO: Get span from the type itself
        let span_end = self.get_span_end(old_name);

        arena.alloc(Alias {
            span_start,
            span_end,
            new_name,
            old_name,
        })
        // self.create_node(AstNode::Alias { new_name, old_name }, span_start, span_end)
    }

    pub fn is_operator(&mut self) -> bool {
        let (token, span) = self.tokens.peek();

        match token {
            Token::Plus
            | Token::PlusPlus
            | Token::Dash
            | Token::Asterisk
            | Token::ForwardSlash
            | Token::ForwardSlashForwardSlash
            | Token::LessThan
            | Token::LessThanEqual
            | Token::GreaterThan
            | Token::GreaterThanEqual
            | Token::EqualsEquals
            | Token::ExclamationEquals
            | Token::EqualsTilde
            | Token::ExclamationTilde
            | Token::AsteriskAsterisk
            | Token::Equals
            | Token::PlusEquals
            | Token::DashEquals
            | Token::AsteriskEquals
            | Token::ForwardSlashEquals
            | Token::PlusPlusEquals => true,
            Token::Bareword => {
                let op = self.compiler.get_span_contents_manual(span.start, span.end);
                op == b"mod" || op == b"in" || op == b"and" || op == b"xor" || op == b"or"
            }
            _ => false,
        }
    }

    pub fn is_equals(&mut self) -> bool {
        self.tokens.peek_token() == Token::Equals
    }

    pub fn is_comma(&mut self) -> bool {
        self.tokens.peek_token() == Token::Comma
    }

    pub fn is_lcurly(&mut self) -> bool {
        self.tokens.peek_token() == Token::LCurly
    }

    pub fn is_rcurly(&mut self) -> bool {
        self.tokens.peek_token() == Token::RCurly
    }

    pub fn is_lparen(&mut self) -> bool {
        self.tokens.peek_token() == Token::LParen
    }

    pub fn is_rparen(&mut self) -> bool {
        self.tokens.peek_token() == Token::RParen
    }

    pub fn is_lsquare(&mut self) -> bool {
        self.tokens.peek_token() == Token::LSquare
    }

    pub fn is_rsquare(&mut self) -> bool {
        self.tokens.peek_token() == Token::RSquare
    }

    pub fn is_less_than(&mut self) -> bool {
        self.tokens.peek_token() == Token::LessThan
    }

    pub fn is_greater_than(&mut self) -> bool {
        self.tokens.peek_token() == Token::GreaterThan
    }

    pub fn is_pipe(&self) -> bool {
        self.tokens.peek_token() == Token::Pipe
    }

    pub fn is_dollar(&self) -> bool {
        self.tokens.peek_token() == Token::Dollar
    }

    pub fn is_comment(&self) -> bool {
        self.tokens.peek_token() == Token::Comment
    }

    pub fn is_question_mark(&self) -> bool {
        self.tokens.peek_token() == Token::QuestionMark
    }

    pub fn is_thin_arrow(&self) -> bool {
        self.tokens.peek_token() == Token::ThinArrow
    }

    pub fn is_thick_arrow(&self) -> bool {
        self.tokens.peek_token() == Token::ThickArrow
    }

    pub fn is_colon(&self) -> bool {
        self.tokens.peek_token() == Token::Colon
    }

    pub fn is_newline(&self) -> bool {
        self.tokens.peek_token() == Token::Newline
    }

    pub fn is_semicolon(&self) -> bool {
        self.tokens.peek_token() == Token::Semicolon
    }

    pub fn is_dot(&self) -> bool {
        self.tokens.peek_token() == Token::Dot
    }

    pub fn is_dotdot(&self) -> bool {
        self.tokens.peek_token() == Token::DotDot
    }

    pub fn is_coloncolon(&self) -> bool {
        self.tokens.peek_token() == Token::ColonColon
    }

    pub fn is_int(&mut self) -> bool {
        self.tokens.peek_token() == Token::Int
    }

    pub fn is_float(&mut self) -> bool {
        self.tokens.peek_token() == Token::Float
    }

    pub fn is_string(&mut self) -> bool {
        self.tokens.peek_token() == Token::DoubleQuotedString
            || self.tokens.peek_token() == Token::SingleQuotedString
    }

    pub fn is_keyword(&mut self, keyword: &[u8]) -> bool {
        if let (Token::Bareword, span) = self.tokens.peek() {
            self.compiler.get_span_contents_manual(span.start, span.end) == keyword
        } else {
            false
        }
    }

    pub fn is_name(&mut self) -> bool {
        self.tokens.peek_token() == Token::Bareword
    }

    pub fn is_eof(&mut self) -> bool {
        self.tokens.peek_token() == Token::Eof
    }

    pub fn is_horizontal_space(&self) -> bool {
        let span_position = self.tokens.peek_span().start;
        let whitespace: &[u8] = b" \t";

        span_position > 0 && whitespace.contains(&self.compiler.source[span_position - 1])
    }

    pub fn is_expression(&mut self) -> bool {
        self.is_simple_expression()
            || self.is_keyword(b"if")
            || self.is_keyword(b"match")
            || self.is_keyword(b"where")
    }

    pub fn is_simple_expression(&mut self) -> bool {
        self.is_string()
            || self.is_int()
            || self.is_float()
            || self.is_lcurly()
            || self.is_lsquare()
            || self.is_lparen()
            || self.is_dot()
            || self.is_dollar()
            || self.is_keyword(b"true")
            || self.is_keyword(b"false")
            || self.is_keyword(b"null")
            || self.is_name()
    }

    // TODO(bumpalo_rewrite)
    pub fn error_on_node(&mut self, message: impl Into<String>, node_id: NodeId) {
        self.compiler.errors.push(SourceError {
            message: message.into(),
            node_id,
            severity: Severity::Error,
        });
    }

    pub fn error(&mut self, message: impl Into<String>) -> NodeId {
        let (token, span) = self.tokens.peek();

        if token != Token::Eof {
            self.tokens.advance();
        }

        let node_id = self.create_node(AstNode::Garbage, span.start, span.end);
        self.compiler.errors.push(SourceError {
            message: message.into(),
            node_id,
            severity: Severity::Error,
        });

        node_id
    }

    pub fn create_node(&mut self, ast_node: AstNode, span_start: usize, span_end: usize) -> NodeId {
        self.compiler.spans.push(Span {
            start: span_start,
            end: span_end,
        });
        self.compiler.push_node(ast_node)
    }

    pub fn lparen(&mut self) {
        if self.is_lparen() {
            self.tokens.advance();
        } else {
            self.error("expected: left paren '('");
        }
    }

    pub fn rparen(&mut self) {
        if self.is_rparen() {
            self.tokens.advance();
        } else {
            self.error("expected: right paren ')'");
        }
    }

    pub fn lsquare(&mut self) {
        if self.is_lsquare() {
            self.tokens.advance();
        } else {
            self.error("expected: left bracket '['");
        }
    }

    pub fn rsquare(&mut self) {
        if self.is_rsquare() {
            self.tokens.advance();
        } else {
            self.error("expected: right bracket ']'");
        }
    }

    pub fn lcurly(&mut self) {
        if self.is_lcurly() {
            self.tokens.advance();
        } else {
            self.error("expected: left bracket '{'");
        }
    }

    pub fn rcurly(&mut self) {
        if self.is_rcurly() {
            self.tokens.advance();
        } else {
            self.error("expected: right bracket '}'");
        }
    }

    pub fn pipe(&mut self) {
        if self.is_pipe() {
            self.tokens.advance();
        } else {
            self.error("expected: pipe symbol '|'");
        }
    }

    pub fn less_than(&mut self) {
        if self.is_less_than() {
            self.tokens.advance();
        } else {
            self.error("expected: less than/left angle bracket '<'");
        }
    }

    pub fn greater_than(&mut self) {
        if self.is_greater_than() {
            self.tokens.advance();
        } else {
            self.error("expected: greater than/right angle bracket '>'");
        }
    }

    pub fn equals(&mut self) {
        if self.is_equals() {
            self.tokens.advance();
        } else {
            self.error("expected: equals '='");
        }
    }

    pub fn thin_arrow(&mut self) {
        if self.is_thin_arrow() {
            self.tokens.advance();
        } else {
            self.error("expected: thin arrow '->'");
        }
    }

    pub fn colon(&mut self) {
        if self.is_colon() {
            self.tokens.advance();
        } else {
            self.error("expected: colon ':'");
        }
    }

    pub fn comma(&mut self) {
        if self.is_comma() {
            self.tokens.advance();
        } else {
            self.error("expected: comma ','");
        }
    }

    pub fn skip_newlines(&mut self) {
        while self.is_newline() {
            self.tokens.advance();
        }
    }

    fn get_rollback_point(&self) -> RollbackPoint {
        self.compiler.get_rollback_point(self.tokens.pos())
    }

    fn apply_rollback(&mut self, rbp: RollbackPoint) {
        let token_pos = self.compiler.apply_compiler_rollback(rbp);
        self.tokens.set_pos(token_pos);
    }
}
