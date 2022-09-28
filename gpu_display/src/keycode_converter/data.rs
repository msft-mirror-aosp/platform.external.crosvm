// Copyright 2022 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#[derive(Debug, Clone, Copy)]
pub struct MapEntry {
    pub linux_keycode: u16,
    pub xkb: u32,
    pub win: u32,
    pub mac: u32,
}

// Based on Chromium's chromium/chromium/ui/events/keycodes/dom/keycode_converter_data.inc.
pub const KEYCODE_MAP: [MapEntry; 92] = [
    MapEntry {
        linux_keycode: 30,
        xkb: 0x0026,
        win: 0x001e,
        mac: 0x0000,
    },
    MapEntry {
        linux_keycode: 48,
        xkb: 0x0038,
        win: 0x0030,
        mac: 0x000b,
    },
    MapEntry {
        linux_keycode: 46,
        xkb: 0x0036,
        win: 0x002e,
        mac: 0x0008,
    },
    MapEntry {
        linux_keycode: 32,
        xkb: 0x0028,
        win: 0x0020,
        mac: 0x0002,
    },
    MapEntry {
        linux_keycode: 18,
        xkb: 0x001a,
        win: 0x0012,
        mac: 0x000e,
    },
    MapEntry {
        linux_keycode: 33,
        xkb: 0x0029,
        win: 0x0021,
        mac: 0x0003,
    },
    MapEntry {
        linux_keycode: 34,
        xkb: 0x002a,
        win: 0x0022,
        mac: 0x0005,
    },
    MapEntry {
        linux_keycode: 35,
        xkb: 0x002b,
        win: 0x0023,
        mac: 0x0004,
    },
    MapEntry {
        linux_keycode: 23,
        xkb: 0x001f,
        win: 0x0017,
        mac: 0x0022,
    },
    MapEntry {
        linux_keycode: 36,
        xkb: 0x002c,
        win: 0x0024,
        mac: 0x0026,
    },
    MapEntry {
        linux_keycode: 37,
        xkb: 0x002d,
        win: 0x0025,
        mac: 0x0028,
    },
    MapEntry {
        linux_keycode: 38,
        xkb: 0x002e,
        win: 0x0026,
        mac: 0x0025,
    },
    MapEntry {
        linux_keycode: 50,
        xkb: 0x003a,
        win: 0x0032,
        mac: 0x002e,
    },
    MapEntry {
        linux_keycode: 49,
        xkb: 0x0039,
        win: 0x0031,
        mac: 0x002d,
    },
    MapEntry {
        linux_keycode: 24,
        xkb: 0x0020,
        win: 0x0018,
        mac: 0x001f,
    },
    MapEntry {
        linux_keycode: 25,
        xkb: 0x0021,
        win: 0x0019,
        mac: 0x0023,
    },
    MapEntry {
        linux_keycode: 16,
        xkb: 0x0018,
        win: 0x0010,
        mac: 0x000c,
    },
    MapEntry {
        linux_keycode: 19,
        xkb: 0x001b,
        win: 0x0013,
        mac: 0x000f,
    },
    MapEntry {
        linux_keycode: 31,
        xkb: 0x0027,
        win: 0x001f,
        mac: 0x0001,
    },
    MapEntry {
        linux_keycode: 20,
        xkb: 0x001c,
        win: 0x0014,
        mac: 0x0011,
    },
    MapEntry {
        linux_keycode: 22,
        xkb: 0x001e,
        win: 0x0016,
        mac: 0x0020,
    },
    MapEntry {
        linux_keycode: 47,
        xkb: 0x0037,
        win: 0x002f,
        mac: 0x0009,
    },
    MapEntry {
        linux_keycode: 17,
        xkb: 0x0019,
        win: 0x0011,
        mac: 0x000d,
    },
    MapEntry {
        linux_keycode: 45,
        xkb: 0x0035,
        win: 0x002d,
        mac: 0x0007,
    },
    MapEntry {
        linux_keycode: 21,
        xkb: 0x001d,
        win: 0x0015,
        mac: 0x0010,
    },
    MapEntry {
        linux_keycode: 44,
        xkb: 0x0034,
        win: 0x002c,
        mac: 0x0006,
    },
    MapEntry {
        linux_keycode: 2,
        xkb: 0x000a,
        win: 0x0002,
        mac: 0x0012,
    },
    MapEntry {
        linux_keycode: 3,
        xkb: 0x000b,
        win: 0x0003,
        mac: 0x0013,
    },
    MapEntry {
        linux_keycode: 4,
        xkb: 0x000c,
        win: 0x0004,
        mac: 0x0014,
    },
    MapEntry {
        linux_keycode: 5,
        xkb: 0x000d,
        win: 0x0005,
        mac: 0x0015,
    },
    MapEntry {
        linux_keycode: 6,
        xkb: 0x000e,
        win: 0x0006,
        mac: 0x0017,
    },
    MapEntry {
        linux_keycode: 7,
        xkb: 0x000f,
        win: 0x0007,
        mac: 0x0016,
    },
    MapEntry {
        linux_keycode: 8,
        xkb: 0x0010,
        win: 0x0008,
        mac: 0x001a,
    },
    MapEntry {
        linux_keycode: 9,
        xkb: 0x0011,
        win: 0x0009,
        mac: 0x001c,
    },
    MapEntry {
        linux_keycode: 10,
        xkb: 0x0012,
        win: 0x000a,
        mac: 0x0019,
    },
    MapEntry {
        linux_keycode: 11,
        xkb: 0x0013,
        win: 0x000b,
        mac: 0x001d,
    },
    MapEntry {
        linux_keycode: 28,
        xkb: 0x0024,
        win: 0x001c,
        mac: 0x0024,
    },
    MapEntry {
        linux_keycode: 1,
        xkb: 0x0009,
        win: 0x0001,
        mac: 0x0035,
    },
    MapEntry {
        linux_keycode: 14,
        xkb: 0x0016,
        win: 0x000e,
        mac: 0x0033,
    },
    MapEntry {
        linux_keycode: 15,
        xkb: 0x0017,
        win: 0x000f,
        mac: 0x0030,
    },
    MapEntry {
        linux_keycode: 57,
        xkb: 0x0041,
        win: 0x0039,
        mac: 0x0031,
    },
    MapEntry {
        linux_keycode: 12,
        xkb: 0x0014,
        win: 0x000c,
        mac: 0x001b,
    },
    MapEntry {
        linux_keycode: 13,
        xkb: 0x0015,
        win: 0x000d,
        mac: 0x0018,
    },
    MapEntry {
        linux_keycode: 26,
        xkb: 0x0022,
        win: 0x001a,
        mac: 0x0021,
    },
    MapEntry {
        linux_keycode: 27,
        xkb: 0x0023,
        win: 0x001b,
        mac: 0x001e,
    },
    MapEntry {
        linux_keycode: 43,
        xkb: 0x0033,
        win: 0x002b,
        mac: 0x002a,
    },
    MapEntry {
        linux_keycode: 39,
        xkb: 0x002f,
        win: 0x0027,
        mac: 0x0029,
    },
    MapEntry {
        linux_keycode: 40,
        xkb: 0x0030,
        win: 0x0028,
        mac: 0x0027,
    },
    MapEntry {
        linux_keycode: 41,
        xkb: 0x0031,
        win: 0x0029,
        mac: 0x0032,
    },
    MapEntry {
        linux_keycode: 51,
        xkb: 0x003b,
        win: 0x0033,
        mac: 0x002b,
    },
    MapEntry {
        linux_keycode: 52,
        xkb: 0x003c,
        win: 0x0034,
        mac: 0x002f,
    },
    MapEntry {
        linux_keycode: 53,
        xkb: 0x003d,
        win: 0x0035,
        mac: 0x002c,
    },
    MapEntry {
        linux_keycode: 58,
        xkb: 0x0042,
        win: 0x003a,
        mac: 0x0039,
    },
    MapEntry {
        linux_keycode: 119,
        xkb: 0x007f,
        win: 0x0045,
        mac: 0xffff,
    },
    MapEntry {
        linux_keycode: 110,
        xkb: 0x0076,
        win: 0xe052,
        mac: 0x0072,
    },
    MapEntry {
        linux_keycode: 102,
        xkb: 0x006e,
        win: 0xe047,
        mac: 0x0073,
    },
    MapEntry {
        linux_keycode: 104,
        xkb: 0x0070,
        win: 0xe049,
        mac: 0x0074,
    },
    MapEntry {
        linux_keycode: 111,
        xkb: 0x0077,
        win: 0xe053,
        mac: 0x0075,
    },
    MapEntry {
        linux_keycode: 107,
        xkb: 0x0073,
        win: 0xe04f,
        mac: 0x0077,
    },
    MapEntry {
        linux_keycode: 109,
        xkb: 0x0075,
        win: 0xe051,
        mac: 0x0079,
    },
    MapEntry {
        linux_keycode: 106,
        xkb: 0x0072,
        win: 0xe04d,
        mac: 0x007c,
    },
    MapEntry {
        linux_keycode: 105,
        xkb: 0x0071,
        win: 0xe04b,
        mac: 0x007b,
    },
    MapEntry {
        linux_keycode: 108,
        xkb: 0x0074,
        win: 0xe050,
        mac: 0x007d,
    },
    MapEntry {
        linux_keycode: 103,
        xkb: 0x006f,
        win: 0xe048,
        mac: 0x007e,
    },
    MapEntry {
        linux_keycode: 69,
        xkb: 0x004d,
        win: 0xe045,
        mac: 0x0047,
    },
    MapEntry {
        linux_keycode: 98,
        xkb: 0x006a,
        win: 0xe035,
        mac: 0x004b,
    },
    MapEntry {
        linux_keycode: 78,
        xkb: 0x0056,
        win: 0x004e,
        mac: 0x0045,
    },
    MapEntry {
        linux_keycode: 28,
        xkb: 0x0068,
        win: 0xe01c,
        mac: 0x004c,
    },
    MapEntry {
        linux_keycode: 79,
        xkb: 0x0057,
        win: 0x004f,
        mac: 0x0053,
    },
    MapEntry {
        linux_keycode: 80,
        xkb: 0x0058,
        win: 0x0050,
        mac: 0x0054,
    },
    MapEntry {
        linux_keycode: 81,
        xkb: 0x0059,
        win: 0x0051,
        mac: 0x0055,
    },
    MapEntry {
        linux_keycode: 75,
        xkb: 0x0053,
        win: 0x004b,
        mac: 0x0056,
    },
    MapEntry {
        linux_keycode: 76,
        xkb: 0x0054,
        win: 0x004c,
        mac: 0x0057,
    },
    MapEntry {
        linux_keycode: 77,
        xkb: 0x0055,
        win: 0x004d,
        mac: 0x0058,
    },
    MapEntry {
        linux_keycode: 71,
        xkb: 0x004f,
        win: 0x0047,
        mac: 0x0059,
    },
    MapEntry {
        linux_keycode: 72,
        xkb: 0x0050,
        win: 0x0048,
        mac: 0x005b,
    },
    MapEntry {
        linux_keycode: 73,
        xkb: 0x0051,
        win: 0x0049,
        mac: 0x005c,
    },
    MapEntry {
        linux_keycode: 82,
        xkb: 0x005a,
        win: 0x0052,
        mac: 0x0052,
    },
    MapEntry {
        linux_keycode: 116,
        xkb: 0x007c,
        win: 0xe05e,
        mac: 0xffff,
    },
    MapEntry {
        linux_keycode: 121,
        xkb: 0x0081,
        win: 0x007e,
        mac: 0x005f,
    },
    MapEntry {
        linux_keycode: 29,
        xkb: 0x0025,
        win: 0x001d,
        mac: 0x003b,
    },
    MapEntry {
        linux_keycode: 42,
        xkb: 0x0032,
        win: 0x002a,
        mac: 0x0038,
    },
    MapEntry {
        linux_keycode: 56,
        xkb: 0x0040,
        win: 0x0038,
        mac: 0x003a,
    },
    MapEntry {
        linux_keycode: 125,
        xkb: 0x0085,
        win: 0xe05b,
        mac: 0x0037,
    },
    MapEntry {
        linux_keycode: 97,
        xkb: 0x0069,
        win: 0xe01d,
        mac: 0x003e,
    },
    MapEntry {
        linux_keycode: 54,
        xkb: 0x003e,
        win: 0x0036,
        mac: 0x003c,
    },
    MapEntry {
        linux_keycode: 100,
        xkb: 0x006c,
        win: 0xe038,
        mac: 0x003d,
    },
    MapEntry {
        linux_keycode: 126,
        xkb: 0x0086,
        win: 0xe05c,
        mac: 0x0036,
    },
    MapEntry {
        linux_keycode: 87,
        xkb: 0x005f,
        win: 0x0057,
        mac: 0x0067,
    },
    MapEntry {
        linux_keycode: 55,
        xkb: 0x003f,
        win: 0x0037,
        mac: 0x0043,
    },
    MapEntry {
        linux_keycode: 83,
        xkb: 0x005b,
        win: 0x0053,
        mac: 0x0041,
    },
    MapEntry {
        linux_keycode: 74,
        xkb: 0x0052,
        win: 0x004a,
        mac: 0x004e,
    },
];
