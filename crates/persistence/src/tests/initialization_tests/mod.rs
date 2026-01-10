// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::SqlitePersistence;

#[test]
fn test_persistence_initialization() {
    let result: Result<SqlitePersistence, crate::error::PersistenceError> =
        SqlitePersistence::new_in_memory();
    assert!(result.is_ok());
}
