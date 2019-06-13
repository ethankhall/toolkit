use pest::iterators::{Pair, Pairs};
use pest::Parser;

use crate::commands::CliError;
#[cfg(test)]
use pest::{consumes_to, parses_to};

#[derive(Parser)]
#[grammar = "commands/json/sql/sql.pest"] // relative to src
struct SqlParser;

#[derive(PartialEq, Debug)]
pub struct Expression {
    pub operation: SqlOperation,
    pub source: SqlSource,
    pub filter: Vec<SqlFilter>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

impl Expression {
    fn new() -> Self {
        Expression {
            operation: SqlOperation::None,
            source: SqlSource::None,
            filter: Vec::new(),
            limit: None,
            offset: None,
        }
    }

    pub fn from(sql: &str) -> Result<Self, CliError> {
        let mut expression = Expression::new();
        let mut pairs = match SqlParser::parse(Rule::bool_expr, sql) {
            Ok(pairs) => pairs,
            Err(e) => {
                return Err(CliError::new(format!("{}", e), 5));
            }
        };

        extract(&mut expression, &mut pairs);

        Ok(expression)
    }
}

#[derive(PartialEq, Debug)]
pub enum SqlOperation {
    None,
    Select { columns: Vec<String> },
}

#[derive(PartialEq, Debug)]
pub enum SqlSource {
    None,
    SqlFrom { path: String },
}

#[derive(PartialEq, Debug)]
pub enum SqlFilter {
    Condition(SqlComparison),
    Or,
    And,
}

#[derive(PartialEq, Debug)]
pub enum SqlComparison {
    Equal { path: String, value: String },
    NotEqual { path: String, value: String },
}

fn extract(expression: &mut Expression, bool_expr: &mut Pairs<Rule>) {
    let mut exp = bool_expr.next().unwrap().into_inner();
    let pairs = exp.next().unwrap().into_inner();
    for pair in pairs {
        trace!("Element: {:?}", pair);
        match pair.as_rule() {
            Rule::select_operation => {
                handle_select(expression, pair);
            }
            Rule::source => {
                handle_from(expression, pair);
            }
            Rule::filter => {
                handle_where(expression, pair);
            }
            Rule::limit => {}
            Rule::offset => {}
            _ => panic!("SQL Parse error! {:?}", pair),
        }
    }
}

fn handle_select(expression: &mut Expression, parent: Pair<Rule>) {
    let inner = parent.into_inner();

    let columns: Vec<String> = inner.map(|x| s!(x.as_span().as_str())).collect();
    expression.operation = SqlOperation::Select { columns: columns };
}

fn handle_from(expression: &mut Expression, parent: Pair<Rule>) {
    expression.source = SqlSource::SqlFrom {
        path: s!(parent.into_inner().as_str()),
    };
}

fn handle_where(expression: &mut Expression, parent: Pair<Rule>) {
    let mut filters: Vec<SqlFilter> = Vec::new();

    for pair in parent.into_inner() {
        match pair.as_rule() {
            Rule::condition => {
                filters.push(handle_condition(&mut pair.into_inner()));
            }
            Rule::logic => match pair.into_inner().next().unwrap().as_rule() {
                Rule::and => {
                    filters.push(SqlFilter::And);
                }
                Rule::or => {
                    filters.push(SqlFilter::Or);
                }
                _ => {
                    panic!("Unknown operator");
                }
            },
            _ => panic!("Unable to parse where"),
        }
    }

    expression.filter = filters;
}

fn handle_condition(parent: &mut Pairs<Rule>) -> SqlFilter {
    let path = s!(parent.next().unwrap().as_str());
    let operator = parent.next().unwrap();
    let value = s!(parent.next().unwrap().as_str());

    let comparison = match operator.as_rule() {
        Rule::eq => SqlComparison::Equal {
            path: path,
            value: value,
        },
        Rule::neq => SqlComparison::NotEqual {
            path: path,
            value: value,
        },
        _ => panic!(
            "Value must be `=` or `!=`, but was {:?} ({:?})",
            operator.as_str(),
            operator.as_rule()
        ),
    };

    SqlFilter::Condition(comparison)
}

#[test]
fn validate_ast_builder() {
    let mut pairs =
        SqlParser::parse(Rule::bool_expr, "select * from . where .a.b.c = 123").unwrap();
    let mut expression = Expression::new();

    extract(&mut expression, &mut pairs);

    assert_eq!(
        SqlOperation::Select {
            columns: vec![s!("*")]
        },
        expression.operation
    );
    assert_eq!(SqlSource::SqlFrom { path: s!(".") }, expression.source);
    assert_eq!(
        vec![SqlFilter::Condition(SqlComparison::Equal {
            path: s!(".a.b.c"),
            value: s!("123")
        })],
        expression.filter
    );
}

#[test]
fn validate_parsing() {
    parses_to! {
        parser: SqlParser,
        input: "select * from . where .a.b.c = 123",
        rule: Rule::expr,
        tokens: [
            expr(0, 34, [
                select_operation(0,9,[
                    result_column(7,8, [])
                ]),
                source(9, 15, [
                    json_path(14, 15, []),
                ]),
                filter(16, 34, [
                    condition(22, 34, [
                        json_path(22, 28, []),
                        eq(29, 30, []),
                        value(31, 34, [num_literal(31, 34, [])])
                    ])
                ])
            ])
        ]
    }

    parses_to! {
        parser: SqlParser,
        input: "select .abc from .a.b.c",
        rule: Rule::expr,
        tokens: [
            expr(0, 23, [
                select_operation(0, 12,[
                    result_column(7, 11, [
                        json_path(7, 11, [])
                    ])
                ]),
                source(12, 23, [
                    json_path(17, 23, []),
                ])
            ])
        ]
    }

    parses_to! {
        parser: SqlParser,
        input: "select .abc from .a.b.c limit 10 offset 10",
        rule: Rule::expr,
        tokens: [
            expr(0, 42, [
                select_operation(0, 12,[
                    result_column(7, 11, [
                        json_path(7, 11, [])
                    ])
                ]),
                source(12, 23, [
                    json_path(17, 23, []),
                ]),
                limit(24, 32, [
                    pos_num_literal(30, 32, []),
                ]),
                offset(33, 42, [
                    pos_num_literal(40, 42, []),
                ])
            ])
        ]
    }

    parses_to! {
        parser: SqlParser,
        input: "select .abc from .a.b.c limit 10",
        rule: Rule::expr,
        tokens: [
            expr(0, 32, [
                select_operation(0, 12,[
                    result_column(7, 11, [
                        json_path(7, 11, [])
                    ])
                ]),
                source(12, 23, [
                    json_path(17, 23, []),
                ]),
                limit(24, 32, [
                    pos_num_literal(30, 32, []),
                ])
            ])
        ]
    }
}
