//! Bot patrol waypoint data for PK zones.
//! `MoveProcessArdreamLandTown()`. Each zone has pre-defined patrol routes
//! with nation-specific coordinates. Bots randomly select a route on spawn
//! or after completing a route.
//! Data format: each waypoint is `(karus_x, karus_z, elmo_x, elmo_z)`.
//! A `(0, 0)` coordinate means the waypoint is invalid for that nation.

use crate::world::types::{ZONE_ARDREAM, ZONE_RONARK_LAND};
use crate::world::NATION_KARUS;

// ── Route count per zone ────────────────────────────────────────────────

/// Number of patrol routes for Ronark Land.
const RONARK_ROUTE_COUNT: u8 = 10;

/// Compact Ronark bowl circuit. Runtime PK bots use these points instead of
/// repeatedly travelling from one nation base to the other. Route ID rotates
/// the starting point and nations walk the loop in opposite directions.
const RONARK_BOWL: [(i16, i16); 12] = [
    (1024, 850),
    (1110, 875),
    (1170, 935),
    (1195, 1024),
    (1170, 1110),
    (1110, 1170),
    (1024, 1195),
    (935, 1170),
    (875, 1110),
    (850, 1024),
    (875, 935),
    (935, 875),
];

/// Number of patrol routes for Ardream.
const ARDREAM_ROUTE_COUNT: u8 = 10;

// ── Max waypoint counts per route (Ronark Land) ─────────────────────────
//
// Format: (karus_max, elmo_max)

const RONARK_MAX: [(u8, u8); 10] = [
    (19, 19), // Route 1
    (34, 29), // Route 2
    (26, 31), // Route 3
    (19, 34), // Route 4
    (21, 39), // Route 5
    (17, 34), // Route 6
    (22, 25), // Route 7
    (16, 24), // Route 8
    (21, 24), // Route 9
    (23, 30), // Route 10
];

// ── Max waypoint counts per route (Ardream) ─────────────────────────────
//

const ARDREAM_MAX: [(u8, u8); 10] = [
    (14, 14), // Route 1
    (14, 14), // Route 2
    (13, 13), // Route 3
    (13, 13), // Route 4
    (14, 14), // Route 5
    (14, 14), // Route 6
    (14, 14), // Route 7
    (14, 14), // Route 8
    (14, 14), // Route 9
    (14, 14), // Route 10
];

// ── Ronark Land Route Data ──────────────────────────────────────────────
// (karus_x, karus_z, elmo_x, elmo_z)

const RONARK_R1: &[(i16, i16, i16, i16)] = &[
    (1375, 1099, 623, 902),
    (1276, 1056, 668, 917),
    (1212, 901, 731, 938),
    (1088, 771, 770, 1007),
    (966, 876, 820, 1078),
    (745, 937, 863, 1140),
    (726, 921, 910, 1166),
    (734, 1016, 939, 1201),
    (574, 1056, 1019, 1207),
    (463, 910, 1067, 1206),
    (517, 880, 1098, 1215),
    (538, 792, 1127, 1182),
    (693, 794, 1201, 1145),
    (772, 941, 1229, 1094),
    (725, 946, 1267, 1059),
    (717, 918, 1275, 1034),
    (775, 944, 1243, 1041),
    (735, 956, 1227, 1100),
    (718, 928, 1267, 978),
];

const RONARK_R2: &[(i16, i16, i16, i16)] = &[
    (1377, 1100, 623, 902),
    (1350, 1087, 647, 920),
    (1311, 1035, 689, 960),
    (1205, 916, 728, 989),
    (1164, 968, 776, 1014),
    (994, 973, 801, 1080),
    (949, 1047, 818, 1142),
    (871, 1049, 876, 1196),
    (843, 1103, 922, 1214),
    (613, 1055, 952, 1244),
    (432, 1118, 1024, 1251),
    (374, 1042, 1010, 1207),
    (392, 957, 1054, 1200),
    (322, 896, 1138, 1233),
    (143, 887, 1155, 1174),
    (282, 695, 1204, 1145),
    (364, 604, 1226, 1097),
    (279, 518, 1247, 1057),
    (193, 590, 1286, 1066),
    (129, 674, 1246, 1055),
    (182, 595, 1267, 1044),
    (303, 513, 1309, 989),
    (403, 645, 1316, 942),
    (506, 735, 1381, 952),
    (586, 782, 1435, 968),
    (709, 811, 1466, 988),
    (764, 933, 1484, 1066),
    (726, 945, 1508, 1133),
    (732, 923, 1460, 1108),
    (776, 951, 0, 0),
    (712, 937, 0, 0),
    (746, 928, 0, 0),
    (703, 949, 0, 0),
    (776, 937, 0, 0),
];

const RONARK_R3: &[(i16, i16, i16, i16)] = &[
    (1380, 1102, 624, 901),
    (1269, 1058, 661, 906),
    (1165, 1173, 685, 899),
    (1034, 1196, 710, 917),
    (952, 1209, 747, 923),
    (799, 1067, 770, 952),
    (738, 935, 795, 1022),
    (715, 915, 836, 1091),
    (740, 955, 862, 1077),
    (776, 944, 876, 1052),
    (726, 946, 897, 1079),
    (768, 934, 927, 1063),
    (717, 919, 963, 1081),
    (749, 966, 992, 1112),
    (708, 1022, 1031, 1144),
    (573, 1065, 1075, 1151),
    (469, 909, 1117, 1160),
    (541, 807, 1137, 1158),
    (688, 795, 1179, 1150),
    (773, 930, 1209, 1125),
    (731, 941, 1223, 1088),
    (723, 923, 1244, 1059),
    (742, 947, 1251, 984),
    (724, 938, 1257, 942),
    (722, 921, 1301, 936),
    (750, 934, 1344, 931),
    (0, 0, 1308, 972),
    (0, 0, 1270, 1052),
    (0, 0, 1282, 1070),
    (0, 0, 1217, 1100),
    (0, 0, 1266, 1062),
];

const RONARK_R4: &[(i16, i16, i16, i16)] = &[
    (1380, 1103, 626, 901),
    (1460, 1081, 638, 912),
    (1589, 1067, 653, 919),
    (1626, 1011, 675, 949),
    (1794, 1185, 710, 930),
    (1521, 1573, 762, 919),
    (1368, 1483, 795, 912),
    (1136, 1265, 830, 843),
    (1060, 1201, 869, 819),
    (869, 1210, 892, 806),
    (793, 1069, 929, 806),
    (764, 937, 989, 795),
    (737, 923, 1060, 798),
    (714, 934, 1142, 815),
    (758, 960, 1186, 843),
    (742, 923, 1194, 888),
    (726, 955, 1223, 906),
    (802, 903, 1227, 933),
    (732, 935, 1250, 982),
    (0, 0, 1274, 1007),
    (0, 0, 1266, 1060),
    (0, 0, 1204, 1129),
    (0, 0, 1178, 1170),
    (0, 0, 1119, 1185),
    (0, 0, 1102, 1216),
    (0, 0, 1154, 1240),
    (0, 0, 1176, 1301),
    (0, 0, 1221, 1349),
    (0, 0, 1264, 1307),
    (0, 0, 1347, 1449),
    (0, 0, 1381, 1488),
    (0, 0, 1428, 1530),
    (0, 0, 1491, 1532),
    (0, 0, 1425, 1572),
];

const RONARK_R5: &[(i16, i16, i16, i16)] = &[
    (1375, 1102, 627, 903),
    (1257, 1045, 636, 913),
    (1218, 897, 623, 893),
    (1166, 965, 628, 896),
    (1030, 972, 634, 925),
    (1020, 1023, 647, 925),
    (866, 1062, 672, 921),
    (838, 1098, 713, 927),
    (721, 925, 728, 910),
    (773, 931, 777, 915),
    (723, 953, 802, 892),
    (918, 767, 833, 844),
    (1017, 805, 861, 831),
    (1056, 923, 886, 803),
    (1001, 972, 906, 776),
    (944, 1051, 952, 764),
    (872, 1056, 996, 769),
    (836, 1096, 1036, 750),
    (726, 943, 1088, 744),
    (771, 938, 1118, 751),
    (716, 928, 1125, 763),
    (0, 0, 1032, 793),
    (0, 0, 1177, 840),
    (0, 0, 1233, 865),
    (0, 0, 1301, 918),
    (0, 0, 1270, 970),
    (0, 0, 1257, 1011),
    (0, 0, 1272, 1059),
    (0, 0, 1282, 1076),
    (0, 0, 1269, 1119),
    (0, 0, 1281, 1165),
    (0, 0, 1316, 1180),
    (0, 0, 1353, 1198),
    (0, 0, 1417, 1196),
    (0, 0, 1469, 1187),
    (0, 0, 1499, 1143),
    (0, 0, 1505, 1085),
    (0, 0, 1460, 1105),
    (0, 0, 1380, 1091),
];

const RONARK_R6: &[(i16, i16, i16, i16)] = &[
    (1375, 1102, 625, 895),
    (1389, 1069, 605, 896),
    (1245, 1081, 605, 925),
    (1100, 1209, 586, 898),
    (1024, 1189, 511, 880),
    (1043, 1032, 485, 902),
    (1050, 971, 485, 933),
    (979, 961, 533, 1010),
    (935, 1045, 558, 1046),
    (868, 1066, 630, 1052),
    (838, 1054, 703, 1073),
    (733, 953, 769, 1083),
    (745, 920, 819, 1086),
    (730, 939, 842, 1104),
    (706, 938, 874, 1054),
    (762, 958, 932, 1050),
    (736, 969, 972, 1037),
    (0, 0, 972, 1037),
    (0, 0, 1059, 986),
    (0, 0, 1099, 959),
    (0, 0, 1146, 965),
    (0, 0, 1175, 971),
    (0, 0, 1190, 927),
    (0, 0, 1215, 915),
    (0, 0, 1243, 959),
    (0, 0, 1248, 1003),
    (0, 0, 1290, 1065),
    (0, 0, 1257, 1060),
    (0, 0, 1264, 1055),
    // C++ bug: case 30 missing, skipped. WP 30 = (0,0) for both.
    (0, 0, 0, 0),
    (0, 0, 1286, 1081),
    (0, 0, 1254, 1045),
    (0, 0, 1300, 1054),
    (0, 0, 1267, 1060),
];

const RONARK_R7: &[(i16, i16, i16, i16)] = &[
    (1376, 1102, 626, 890),
    (1272, 1048, 634, 923),
    (1206, 896, 635, 911),
    (1124, 777, 693, 927),
    (982, 794, 741, 911),
    (832, 728, 791, 910),
    (794, 923, 844, 840),
    (708, 1023, 890, 795),
    (578, 1065, 911, 782),
    (526, 1006, 960, 783),
    (439, 1013, 978, 798),
    (334, 900, 1015, 809),
    (175, 895, 1037, 780),
    (288, 689, 1180, 838),
    (464, 514, 1230, 867),
    (611, 400, 1253, 937),
    (706, 541, 1251, 990),
    (843, 667, 1269, 1024),
    (774, 943, 1269, 1060),
    (729, 954, 1253, 1067),
    (770, 996, 1250, 1055),
    (729, 936, 1275, 1047),
    (0, 0, 1254, 1035),
    (0, 0, 1291, 1073),
    (0, 0, 1242, 1055),
];

const RONARK_R8: &[(i16, i16, i16, i16)] = &[
    (1375, 1091, 622, 901),
    (1487, 1111, 626, 902),
    (1463, 970, 641, 916),
    (1267, 901, 708, 923),
    (1129, 803, 751, 997),
    (836, 773, 800, 1044),
    (728, 839, 827, 1080),
    (754, 930, 842, 1092),
    (728, 942, 860, 1073),
    (768, 930, 887, 1040),
    (735, 951, 936, 1042),
    (765, 977, 971, 1043),
    (768, 936, 993, 1014),
    (730, 947, 1023, 986),
    (783, 946, 1055, 972),
    (731, 950, 1130, 958),
    (0, 0, 1172, 974),
    (0, 0, 1186, 916),
    (0, 0, 1128, 907),
    (0, 0, 1253, 925),
    (0, 0, 1247, 1005),
    (0, 0, 1268, 1025),
    (0, 0, 1260, 1061),
    (0, 0, 1244, 1053),
];

const RONARK_R9: &[(i16, i16, i16, i16)] = &[
    (1376, 1103, 625, 902),
    (1255, 1040, 654, 915),
    (1241, 897, 692, 886),
    (1190, 930, 709, 877),
    (1177, 966, 750, 863),
    (1070, 968, 768, 855),
    (980, 1045, 809, 845),
    (885, 1035, 875, 818),
    (847, 1095, 893, 810),
    (726, 939, 931, 803),
    (805, 914, 979, 795),
    (761, 949, 1012, 813),
    (736, 920, 1025, 850),
    (743, 980, 1052, 887),
    (776, 951, 1037, 914),
    (753, 946, 1060, 928),
    (720, 971, 1102, 951),
    (769, 974, 1138, 962),
    (758, 941, 1173, 965),
    (712, 800, 1189, 940),
    (769, 939, 1199, 922),
    (0, 0, 1219, 917),
    (0, 0, 1256, 961),
    (0, 0, 1271, 1059),
];

const RONARK_R10: &[(i16, i16, i16, i16)] = &[
    (1378, 1102, 626, 903),
    (1340, 1159, 597, 915),
    (1137, 1220, 563, 890),
    (940, 1197, 480, 877),
    (800, 1063, 473, 921),
    (508, 1084, 524, 994),
    (377, 1066, 567, 1047),
    (389, 956, 691, 1070),
    (337, 900, 777, 1085),
    (180, 888, 787, 1121),
    (287, 694, 825, 1148),
    (359, 610, 894, 1198),
    (468, 693, 949, 1204),
    (547, 794, 994, 1201),
    (694, 789, 1026, 1165),
    (777, 940, 1047, 1118),
    (738, 925, 1062, 1096),
    (770, 956, 1106, 1113),
    (740, 962, 1144, 1139),
    (751, 935, 1161, 1151),
    (651, 1061, 1183, 1130),
    (772, 969, 1208, 1121),
    (765, 940, 1248, 1076),
    (0, 0, 1241, 1060),
    (0, 0, 1255, 1045),
    (0, 0, 1250, 1065),
    (0, 0, 1264, 1051),
    (0, 0, 1266, 1060),
    (0, 0, 1257, 1057),
    (0, 0, 1272, 1062),
];

/// All Ronark Land routes (indexed 0–9, routes are 1–10 in C++).
const RONARK_ROUTES: [&[(i16, i16, i16, i16)]; 10] = [
    RONARK_R1, RONARK_R2, RONARK_R3, RONARK_R4, RONARK_R5, RONARK_R6, RONARK_R7, RONARK_R8,
    RONARK_R9, RONARK_R10,
];

// ── Ardream Route Data ──────────────────────────────────────────────────

const ARDREAM_R1: &[(i16, i16, i16, i16)] = &[
    (856, 138, 195, 901),
    (809, 156, 186, 875),
    (739, 297, 214, 762),
    (828, 533, 230, 692),
    (908, 638, 216, 581),
    (864, 777, 292, 544),
    (772, 838, 469, 534),
    (712, 889, 524, 545),
    (548, 895, 614, 507),
    (467, 928, 801, 447),
    (327, 883, 775, 197),
    (234, 824, 686, 238),
    (194, 816, 767, 247),
    (205, 796, 816, 176),
];

const ARDREAM_R2: &[(i16, i16, i16, i16)] = &[
    (853, 141, 188, 899),
    (768, 223, 203, 732),
    (668, 172, 235, 626),
    (490, 148, 132, 447),
    (368, 100, 215, 335),
    (325, 144, 218, 195),
    (212, 190, 336, 132),
    (206, 270, 359, 114),
    (219, 337, 519, 154),
    (100, 474, 637, 211),
    (210, 595, 760, 249),
    (227, 779, 768, 203),
    (189, 803, 799, 220),
    (212, 816, 750, 205),
];

const ARDREAM_R3: &[(i16, i16, i16, i16)] = &[
    (856, 140, 191, 900),
    (800, 136, 126, 876),
    (757, 207, 135, 766),
    (753, 463, 117, 694),
    (626, 539, 267, 540),
    (550, 621, 396, 491),
    (394, 560, 534, 378),
    (249, 542, 715, 497),
    (221, 744, 761, 379),
    (195, 807, 769, 206),
    (250, 817, 788, 220),
    (208, 784, 716, 197),
    (192, 849, 791, 206),
];

const ARDREAM_R4: &[(i16, i16, i16, i16)] = &[
    (852, 141, 192, 899),
    (831, 181, 186, 821),
    (770, 226, 272, 842),
    (810, 453, 396, 915),
    (645, 524, 557, 915),
    (537, 436, 720, 883),
    (478, 423, 789, 822),
    (314, 542, 864, 788),
    (221, 577, 875, 596),
    (216, 697, 812, 502),
    (193, 804, 772, 212),
    (239, 823, 772, 241),
    (198, 819, 789, 215),
];

const ARDREAM_R5: &[(i16, i16, i16, i16)] = &[
    (855, 140, 195, 901),
    (806, 157, 210, 888),
    (762, 209, 223, 813),
    (593, 161, 333, 885),
    (513, 155, 460, 919),
    (371, 106, 723, 880),
    (305, 153, 823, 801),
    (128, 253, 871, 651),
    (160, 391, 756, 453),
    (134, 585, 767, 219),
    (238, 716, 806, 241),
    (225, 808, 669, 168),
    (187, 816, 722, 246),
    (223, 833, 778, 226),
];

const ARDREAM_R6: &[(i16, i16, i16, i16)] = &[
    (853, 139, 189, 902),
    (807, 158, 202, 735),
    (798, 269, 241, 624),
    (824, 400, 376, 586),
    (819, 554, 464, 589),
    (874, 659, 546, 620),
    (852, 784, 702, 491),
    (768, 842, 779, 419),
    (727, 877, 747, 306),
    (467, 931, 786, 200),
    (323, 879, 743, 214),
    (200, 795, 794, 225),
    (209, 819, 779, 188),
    (188, 818, 791, 197),
];

const ARDREAM_R7: &[(i16, i16, i16, i16)] = &[
    (856, 137, 189, 907),
    (834, 144, 189, 839),
    (770, 232, 220, 741),
    (780, 344, 217, 639),
    (730, 484, 100, 478),
    (576, 520, 135, 331),
    (539, 538, 190, 197),
    (463, 534, 325, 145),
    (346, 604, 360, 110),
    (243, 609, 525, 156),
    (208, 740, 606, 154),
    (194, 821, 776, 227),
    (235, 823, 755, 189),
    (195, 792, 802, 210),
];

const ARDREAM_R8: &[(i16, i16, i16, i16)] = &[
    (856, 137, 176, 938),
    (812, 121, 182, 910),
    (768, 184, 192, 853),
    (673, 178, 249, 824),
    (554, 177, 407, 925),
    (367, 97, 598, 908),
    (316, 157, 719, 877),
    (215, 275, 780, 832),
    (238, 383, 863, 790),
    (199, 476, 862, 579),
    (209, 528, 795, 472),
    (205, 691, 779, 265),
    (231, 774, 769, 188),
    (188, 815, 794, 213),
];

const ARDREAM_R9: &[(i16, i16, i16, i16)] = &[
    (853, 140, 194, 899),
    (808, 157, 185, 842),
    (750, 329, 230, 710),
    (828, 544, 198, 561),
    (896, 611, 133, 445),
    (889, 782, 211, 341),
    (783, 825, 209, 197),
    (720, 884, 323, 143),
    (452, 927, 363, 105),
    (337, 885, 519, 154),
    (194, 787, 627, 215),
    (208, 813, 765, 242),
    (240, 770, 799, 209),
    (197, 826, 770, 194),
];

const ARDREAM_R10: &[(i16, i16, i16, i16)] = &[
    (858, 117, 166, 899),
    (784, 122, 169, 856),
    (757, 253, 208, 782),
    (755, 457, 225, 698),
    (668, 504, 239, 622),
    (548, 365, 367, 602),
    (401, 478, 455, 465),
    (328, 541, 529, 457),
    (250, 562, 694, 500),
    (212, 633, 752, 426),
    (227, 777, 773, 196),
    (190, 823, 769, 197),
    (289, 841, 797, 209),
    (208, 798, 783, 182),
];

/// All Ardream routes (indexed 0–9, routes are 1–10 in C++).
const ARDREAM_ROUTES: [&[(i16, i16, i16, i16)]; 10] = [
    ARDREAM_R1,
    ARDREAM_R2,
    ARDREAM_R3,
    ARDREAM_R4,
    ARDREAM_R5,
    ARDREAM_R6,
    ARDREAM_R7,
    ARDREAM_R8,
    ARDREAM_R9,
    ARDREAM_R10,
];

// ── Public API ──────────────────────────────────────────────────────────

/// Select a random patrol route for the given zone.
/// Returns a route ID (1-based) or 0 if the zone has no routes.
pub fn random_route(zone_id: u16) -> u8 {
    let count = match zone_id {
        ZONE_RONARK_LAND => RONARK_ROUTE_COUNT,
        ZONE_ARDREAM => ARDREAM_ROUTE_COUNT,
        _ => return 0,
    };
    let mut rng = rand::thread_rng();
    rng.gen_range(1..=count)
}

/// Get the maximum waypoint count for a route in the given zone/nation.
/// Returns the max waypoint index (1-based) for the given route and nation.
pub fn route_max_waypoints(zone_id: u16, route: u8, nation: u8) -> u8 {
    let maxes = match zone_id {
        ZONE_RONARK_LAND => &RONARK_MAX[..],
        ZONE_ARDREAM => &ARDREAM_MAX[..],
        _ => return 0,
    };
    let idx = route.saturating_sub(1) as usize;
    if idx >= maxes.len() {
        return 0;
    }
    let (karus_max, elmo_max) = maxes[idx];
    if nation == NATION_KARUS {
        karus_max
    } else {
        elmo_max
    }
}

/// Get the waypoint coordinates for a specific route/state/nation.
/// `route` is 1-based, `state` is 1-based (matching C++ m_MoveState).
/// Returns `Some((x, z))` or `None` if the waypoint is invalid (0,0) or
/// the state/route is out of range.
pub fn get_waypoint(zone_id: u16, route: u8, state: u8, nation: u8) -> Option<(f32, f32)> {
    let routes = match zone_id {
        ZONE_RONARK_LAND => &RONARK_ROUTES[..],
        ZONE_ARDREAM => &ARDREAM_ROUTES[..],
        _ => return None,
    };

    let route_idx = route.checked_sub(1)? as usize;
    if route_idx >= routes.len() {
        return None;
    }

    let waypoints = routes[route_idx];
    let state_idx = state.checked_sub(1)? as usize;
    if state_idx >= waypoints.len() {
        return None;
    }

    let (kx, kz, ex, ez) = waypoints[state_idx];
    let (x, z) = if nation == NATION_KARUS {
        (kx, kz)
    } else {
        (ex, ez)
    };

    // (0, 0) means the waypoint is invalid for this nation.
    if x == 0 && z == 0 {
        return None;
    }

    Some((x as f32, z as f32))
}

/// Return a smooth Ronark bowl patrol point. Route rotates the starting point
/// so a wave does not stack, while nations walk the circuit in opposite
/// directions and naturally encounter each other.
pub fn get_bowl_waypoint(route: u8, state: u8, nation: u8) -> Option<(f32, f32)> {
    let len = RONARK_BOWL.len();
    let route_offset = route.saturating_sub(1) as usize % len;
    let state_offset = state.checked_sub(1)? as usize;
    if state_offset >= len {
        return None;
    }
    let forward = (route_offset + state_offset) % len;
    let index = if nation == NATION_KARUS {
        forward
    } else {
        (len - forward) % len
    };
    let (x, z) = RONARK_BOWL[index];
    Some((x as f32, z as f32))
}

pub const fn bowl_waypoint_count() -> u8 {
    RONARK_BOWL.len() as u8
}

use rand::Rng;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::pathfind::find_bot_path;
    use crate::zone::MapData;
    use ko_protocol::smd::SmdFile;
    use std::path::Path;

    #[test]
    fn ronark_pk_patrol_points_can_be_projected_to_safe_reachable_tiles() {
        let smd_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Map/freezone_b.smd");
        let map = MapData::new(SmdFile::load(&smd_path).expect("load Ronark Land SMD"));
        let starts = [(1050.0, 842.0), (886.0, 1102.0)];
        for (x, z) in starts {
            let (mut from_x, mut from_z) =
                crate::systems::pathfind::nearest_bot_point(&map, x, z, 8)
                    .expect("find a walkable Ronark bowl spawn tile");
            let mut reachable_points = 0;
            for &(raw_x, raw_z) in &RONARK_BOWL {
                let (goal_x, goal_z) = crate::systems::pathfind::nearest_bot_point(
                    &map,
                    raw_x as f32,
                    raw_z as f32,
                    8,
                )
                .expect("find a safe tile near every patrol point");
                let result = find_bot_path(&map, from_x, from_z, goal_x, goal_z);
                if result.found {
                    reachable_points += 1;
                    from_x = goal_x;
                    from_z = goal_z;
                }
            }
            assert!(reachable_points >= 4, "only {reachable_points} Ronark patrol destinations are reachable from spawn ({x},{z})");
        }
    }

    #[test]
    fn test_random_route_ronark() {
        for _ in 0..50 {
            let r = random_route(ZONE_RONARK_LAND);
            assert!((1..=10).contains(&r), "route {} out of range", r);
        }
    }

    #[test]
    fn test_random_route_ardream() {
        for _ in 0..50 {
            let r = random_route(ZONE_ARDREAM);
            assert!((1..=10).contains(&r), "route {} out of range", r);
        }
    }

    #[test]
    fn test_random_route_non_pk_zone() {
        assert_eq!(random_route(21), 0, "non-PK zone should return 0");
    }

    #[test]
    fn test_route_max_waypoints_ronark() {
        assert_eq!(route_max_waypoints(ZONE_RONARK_LAND, 1, 1), 19);
        assert_eq!(route_max_waypoints(ZONE_RONARK_LAND, 1, 2), 19);
        assert_eq!(route_max_waypoints(ZONE_RONARK_LAND, 2, 1), 34);
        assert_eq!(route_max_waypoints(ZONE_RONARK_LAND, 2, 2), 29);
        assert_eq!(route_max_waypoints(ZONE_RONARK_LAND, 5, 2), 39);
    }

    #[test]
    fn test_route_max_waypoints_ardream() {
        assert_eq!(route_max_waypoints(ZONE_ARDREAM, 1, 1), 14);
        assert_eq!(route_max_waypoints(ZONE_ARDREAM, 3, 1), 13);
        assert_eq!(route_max_waypoints(ZONE_ARDREAM, 3, 2), 13);
    }

    #[test]
    fn test_get_waypoint_ronark_route1_karus() {
        let wp = get_waypoint(ZONE_RONARK_LAND, 1, 1, 1);
        assert_eq!(wp, Some((1375.0, 1099.0)));
    }

    #[test]
    fn test_get_waypoint_ronark_route1_elmo() {
        let wp = get_waypoint(ZONE_RONARK_LAND, 1, 1, 2);
        assert_eq!(wp, Some((623.0, 902.0)));
    }

    #[test]
    fn test_get_waypoint_ronark_route1_last() {
        let wp = get_waypoint(ZONE_RONARK_LAND, 1, 19, 1);
        assert_eq!(wp, Some((718.0, 928.0)));
    }

    #[test]
    fn test_get_waypoint_invalid_nation_returns_none() {
        // Route 2, state 30: KARUS has valid data, ELMORAD has (0,0)
        let wp = get_waypoint(ZONE_RONARK_LAND, 2, 30, 2);
        assert_eq!(wp, None, "ELMORAD WP 30 on route 2 should be None (0,0)");
    }

    #[test]
    fn test_get_waypoint_out_of_range() {
        // State 99 doesn't exist
        assert_eq!(get_waypoint(ZONE_RONARK_LAND, 1, 99, 1), None);
        // Route 99 doesn't exist
        assert_eq!(get_waypoint(ZONE_RONARK_LAND, 99, 1, 1), None);
        // State 0 is invalid (1-based)
        assert_eq!(get_waypoint(ZONE_RONARK_LAND, 1, 0, 1), None);
    }

    #[test]
    fn test_get_waypoint_ardream_route1_karus() {
        let wp = get_waypoint(ZONE_ARDREAM, 1, 1, 1);
        assert_eq!(wp, Some((856.0, 138.0)));
    }

    #[test]
    fn test_get_waypoint_ardream_route1_elmo() {
        let wp = get_waypoint(ZONE_ARDREAM, 1, 1, 2);
        assert_eq!(wp, Some((195.0, 901.0)));
    }

    #[test]
    fn test_get_waypoint_ardream_route3_max() {
        // Route 3 has 13 waypoints
        let wp = get_waypoint(ZONE_ARDREAM, 3, 13, 1);
        assert_eq!(wp, Some((192.0, 849.0)));
        // State 14 doesn't exist for route 3
        assert_eq!(get_waypoint(ZONE_ARDREAM, 3, 14, 1), None);
    }

    #[test]
    fn test_ronark_route6_bug_parity() {
        // C++ bug: case 30 is missing in route 6 — should be (0,0) for both nations
        let karus = get_waypoint(ZONE_RONARK_LAND, 6, 30, 1);
        let elmo = get_waypoint(ZONE_RONARK_LAND, 6, 30, 2);
        assert_eq!(karus, None, "route 6 WP 30 should be None (C++ bug parity)");
        assert_eq!(elmo, None, "route 6 WP 30 should be None (C++ bug parity)");
    }

    #[test]
    fn test_all_routes_have_valid_first_waypoint() {
        for route in 1..=10 {
            for nation in [1u8, 2u8] {
                let wp = get_waypoint(ZONE_RONARK_LAND, route, 1, nation);
                assert!(
                    wp.is_some(),
                    "Ronark route {} nation {} WP 1 should exist",
                    route,
                    nation
                );
                if route <= 10 {
                    let wp2 = get_waypoint(ZONE_ARDREAM, route, 1, nation);
                    assert!(
                        wp2.is_some(),
                        "Ardream route {} nation {} WP 1 should exist",
                        route,
                        nation
                    );
                }
            }
        }
    }
}
