// Copyright 2020 <盏一 w@hidva.com>
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::clog::{XidStatus, XACTS_PER_BYTE};
use crate::guc;
use crate::utils::Xid;
use crate::KB_BLCKSZ;
use std::assert_eq;

#[test]
fn t() {
    let sess = super::new_session();
    assert_eq!(1, guc::get_int(&sess.gucstate, guc::ClogL2cacheSize));

    let xid = Xid::new(1).unwrap();
    assert_eq!(XidStatus::InProgress, sess.clog.xid_status(xid).unwrap());
    sess.clog.set_xid_status(xid, XidStatus::Committed).unwrap();
    assert_eq!(XidStatus::Committed, sess.clog.xid_status(xid).unwrap());

    let xid = Xid::new(KB_BLCKSZ as u64 * XACTS_PER_BYTE * 4).unwrap();
    assert_eq!(XidStatus::InProgress, sess.clog.xid_status(xid).unwrap());
    sess.clog.set_xid_status(xid, XidStatus::Committed).unwrap();
    assert_eq!(XidStatus::Committed, sess.clog.xid_status(xid).unwrap());

    let xid = Xid::new(1).unwrap();
    assert_eq!(XidStatus::Committed, sess.clog.xid_status(xid).unwrap());
}
