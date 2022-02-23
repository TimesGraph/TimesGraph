/*******************************************************************************
 *
 *
 *  Copyright (c) 2019-2022 TimesGraph
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *  http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 *
 ******************************************************************************/

use super::*;

#[test]
fn create_index() {
    let statements = QUERY_PARSER.parse("create index index_name on table_name (col_1, col_2);");

    assert_eq!(
        statements,
        Ok(Request::Statement(Statement::Definition(Definition::CreateIndex {
            name: "index_name".to_owned(),
            schema_name: "public".to_owned(),
            table_name: "table_name".to_owned(),
            column_names: vec!["col_1".to_owned(), "col_2".to_owned()]
        })))
    );
}
