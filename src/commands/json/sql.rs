use pest::error::Error;
use pest::Parser;

use clap::ArgMatches;

use crate::commands::CliError;

use crate::commands::progress::*;

#[derive(Parser)]
#[grammar = "commands/json/sql.pest"] // relative to src
struct SqlParser;

pub fn do_json_filter_command(args: &ArgMatches)-> Result<(), CliError> {
    Ok(())
}

#[test]
fn validate_parsing() {
    parses_to! {
        parser: SqlParser,
        input: "select * from . where .a.b.c = 123",
        rule: Rule::expr,
        tokens: [
            expr(0, 34, [
                operation(0,9, [
                    select_operation(0,9,[
                        result_column(7,8, [])
                    ])
                ]),
                source(9, 15, [
                    json_path(14, 15, []),
                ]),
                filter(16, 34, [
                    json_path(22, 28, []),
                    comparison(29, 30, [
                        eq(29, 30, [])
                    ]),
                    value(31, 34, [num_literal(31, 34, [])])
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
                operation(0, 12, [
                    select_operation(0, 12,[
                        result_column(7, 11, [
                            json_path(7, 11, [])
                        ])
                    ])
                ]),
                source(12, 23, [
                    json_path(17, 23, []),
                ])
            ])
        ]
    }
}