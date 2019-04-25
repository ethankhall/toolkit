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
        input: "select * from json where .a.b.c = 123",
        rule: Rule::expr,
        tokens: [
            expr(0, 37, [
                operation(0,9, [
                    select_operation(0,9,[
                        result_column(7,8, [])
                    ])
                ]),
                source(9, 18, []),
                filter(19, 37, [
                    json_path(25, 31, []),
                    comparison(32,33, [
                        eq(32, 33, [])
                    ]),
                    value(34, 37, [num_literal(34, 37, [])])
                ])
            ])
        ]
    }

    parses_to! {
        parser: SqlParser,
        input: "select .abc from json",
        rule: Rule::expr,
        tokens: [
            expr(0, 21, [
                operation(0, 12, [
                    select_operation(0, 12,[
                        result_column(7, 11, [
                            json_path(7, 11, [])
                        ])
                    ])
                ]),
                source(12, 21, [])
            ])
        ]
    }
}