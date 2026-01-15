// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Test module to verify if generic Diesel queries work across backends.

use diesel::prelude::*;
use diesel::sql_types::BigInt;

use crate::diesel_schema::operators;
use crate::error::PersistenceError;

/// Test function to see if we can make a generic query work.
///
/// This function attempts to be generic over the connection type
/// while using Diesel DSL.
pub fn count_operators<C>(conn: &mut C) -> Result<i64, PersistenceError>
where
    C: diesel::Connection,
    C::Backend: diesel::backend::Backend,
    operators::table: diesel::query_dsl::methods::SelectDsl<diesel::dsl::CountStar>,
    operators::table: diesel::query_dsl::methods::LoadQuery<'static, C, i64>,
{
    let count: i64 = operators::table.count().get_result(conn)?;
    Ok(count)
}
