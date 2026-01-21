// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/**
 * TypeScript declarations for SCSS modules.
 * This allows importing .scss files as typed modules.
 */

declare module "*.module.scss" {
  const classes: { [key: string]: string };
  export default classes;
}
