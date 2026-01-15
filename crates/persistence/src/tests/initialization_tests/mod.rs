// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::Persistence;

#[test]
fn test_persistence_initialization() {
    let result: Result<Persistence, crate::error::PersistenceError> = Persistence::new_in_memory();
    assert!(result.is_ok());
}
