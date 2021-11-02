// RUN: llvm-mc -triple i386-unknown-unknown --show-encoding %s | FileCheck %s

// CHECK: pabsb -485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x1c,0xa4,0x82,0x10,0xe3,0x0f,0xe3]
pabsb -485498096(%edx,%eax,4), %mm4

// CHECK: pabsb 485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x1c,0xa4,0x82,0xf0,0x1c,0xf0,0x1c]
pabsb 485498096(%edx,%eax,4), %mm4

// CHECK: pabsb -485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x1c,0x8c,0x82,0x10,0xe3,0x0f,0xe3]
pabsb -485498096(%edx,%eax,4), %xmm1

// CHECK: pabsb 485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x1c,0x8c,0x82,0xf0,0x1c,0xf0,0x1c]
pabsb 485498096(%edx,%eax,4), %xmm1

// CHECK: pabsb 485498096(%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x1c,0xa2,0xf0,0x1c,0xf0,0x1c]
pabsb 485498096(%edx), %mm4

// CHECK: pabsb 485498096(%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x1c,0x8a,0xf0,0x1c,0xf0,0x1c]
pabsb 485498096(%edx), %xmm1

// CHECK: pabsb 485498096, %mm4
// CHECK: encoding: [0x0f,0x38,0x1c,0x25,0xf0,0x1c,0xf0,0x1c]
pabsb 485498096, %mm4

// CHECK: pabsb 485498096, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x1c,0x0d,0xf0,0x1c,0xf0,0x1c]
pabsb 485498096, %xmm1

// CHECK: pabsb 64(%edx,%eax), %mm4
// CHECK: encoding: [0x0f,0x38,0x1c,0x64,0x02,0x40]
pabsb 64(%edx,%eax), %mm4

// CHECK: pabsb 64(%edx,%eax), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x1c,0x4c,0x02,0x40]
pabsb 64(%edx,%eax), %xmm1

// CHECK: pabsb (%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x1c,0x22]
pabsb (%edx), %mm4

// CHECK: pabsb (%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x1c,0x0a]
pabsb (%edx), %xmm1

// CHECK: pabsb %mm4, %mm4
// CHECK: encoding: [0x0f,0x38,0x1c,0xe4]
pabsb %mm4, %mm4

// CHECK: pabsb %xmm1, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x1c,0xc9]
pabsb %xmm1, %xmm1

// CHECK: pabsd -485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x1e,0xa4,0x82,0x10,0xe3,0x0f,0xe3]
pabsd -485498096(%edx,%eax,4), %mm4

// CHECK: pabsd 485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x1e,0xa4,0x82,0xf0,0x1c,0xf0,0x1c]
pabsd 485498096(%edx,%eax,4), %mm4

// CHECK: pabsd -485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x1e,0x8c,0x82,0x10,0xe3,0x0f,0xe3]
pabsd -485498096(%edx,%eax,4), %xmm1

// CHECK: pabsd 485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x1e,0x8c,0x82,0xf0,0x1c,0xf0,0x1c]
pabsd 485498096(%edx,%eax,4), %xmm1

// CHECK: pabsd 485498096(%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x1e,0xa2,0xf0,0x1c,0xf0,0x1c]
pabsd 485498096(%edx), %mm4

// CHECK: pabsd 485498096(%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x1e,0x8a,0xf0,0x1c,0xf0,0x1c]
pabsd 485498096(%edx), %xmm1

// CHECK: pabsd 485498096, %mm4
// CHECK: encoding: [0x0f,0x38,0x1e,0x25,0xf0,0x1c,0xf0,0x1c]
pabsd 485498096, %mm4

// CHECK: pabsd 485498096, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x1e,0x0d,0xf0,0x1c,0xf0,0x1c]
pabsd 485498096, %xmm1

// CHECK: pabsd 64(%edx,%eax), %mm4
// CHECK: encoding: [0x0f,0x38,0x1e,0x64,0x02,0x40]
pabsd 64(%edx,%eax), %mm4

// CHECK: pabsd 64(%edx,%eax), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x1e,0x4c,0x02,0x40]
pabsd 64(%edx,%eax), %xmm1

// CHECK: pabsd (%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x1e,0x22]
pabsd (%edx), %mm4

// CHECK: pabsd (%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x1e,0x0a]
pabsd (%edx), %xmm1

// CHECK: pabsd %mm4, %mm4
// CHECK: encoding: [0x0f,0x38,0x1e,0xe4]
pabsd %mm4, %mm4

// CHECK: pabsd %xmm1, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x1e,0xc9]
pabsd %xmm1, %xmm1

// CHECK: pabsw -485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x1d,0xa4,0x82,0x10,0xe3,0x0f,0xe3]
pabsw -485498096(%edx,%eax,4), %mm4

// CHECK: pabsw 485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x1d,0xa4,0x82,0xf0,0x1c,0xf0,0x1c]
pabsw 485498096(%edx,%eax,4), %mm4

// CHECK: pabsw -485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x1d,0x8c,0x82,0x10,0xe3,0x0f,0xe3]
pabsw -485498096(%edx,%eax,4), %xmm1

// CHECK: pabsw 485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x1d,0x8c,0x82,0xf0,0x1c,0xf0,0x1c]
pabsw 485498096(%edx,%eax,4), %xmm1

// CHECK: pabsw 485498096(%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x1d,0xa2,0xf0,0x1c,0xf0,0x1c]
pabsw 485498096(%edx), %mm4

// CHECK: pabsw 485498096(%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x1d,0x8a,0xf0,0x1c,0xf0,0x1c]
pabsw 485498096(%edx), %xmm1

// CHECK: pabsw 485498096, %mm4
// CHECK: encoding: [0x0f,0x38,0x1d,0x25,0xf0,0x1c,0xf0,0x1c]
pabsw 485498096, %mm4

// CHECK: pabsw 485498096, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x1d,0x0d,0xf0,0x1c,0xf0,0x1c]
pabsw 485498096, %xmm1

// CHECK: pabsw 64(%edx,%eax), %mm4
// CHECK: encoding: [0x0f,0x38,0x1d,0x64,0x02,0x40]
pabsw 64(%edx,%eax), %mm4

// CHECK: pabsw 64(%edx,%eax), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x1d,0x4c,0x02,0x40]
pabsw 64(%edx,%eax), %xmm1

// CHECK: pabsw (%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x1d,0x22]
pabsw (%edx), %mm4

// CHECK: pabsw (%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x1d,0x0a]
pabsw (%edx), %xmm1

// CHECK: pabsw %mm4, %mm4
// CHECK: encoding: [0x0f,0x38,0x1d,0xe4]
pabsw %mm4, %mm4

// CHECK: pabsw %xmm1, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x1d,0xc9]
pabsw %xmm1, %xmm1

// CHECK: palignr $0, -485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x3a,0x0f,0xa4,0x82,0x10,0xe3,0x0f,0xe3,0x00]
palignr $0, -485498096(%edx,%eax,4), %mm4

// CHECK: palignr $0, 485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x3a,0x0f,0xa4,0x82,0xf0,0x1c,0xf0,0x1c,0x00]
palignr $0, 485498096(%edx,%eax,4), %mm4

// CHECK: palignr $0, -485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x3a,0x0f,0x8c,0x82,0x10,0xe3,0x0f,0xe3,0x00]
palignr $0, -485498096(%edx,%eax,4), %xmm1

// CHECK: palignr $0, 485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x3a,0x0f,0x8c,0x82,0xf0,0x1c,0xf0,0x1c,0x00]
palignr $0, 485498096(%edx,%eax,4), %xmm1

// CHECK: palignr $0, 485498096(%edx), %mm4
// CHECK: encoding: [0x0f,0x3a,0x0f,0xa2,0xf0,0x1c,0xf0,0x1c,0x00]
palignr $0, 485498096(%edx), %mm4

// CHECK: palignr $0, 485498096(%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x3a,0x0f,0x8a,0xf0,0x1c,0xf0,0x1c,0x00]
palignr $0, 485498096(%edx), %xmm1

// CHECK: palignr $0, 485498096, %mm4
// CHECK: encoding: [0x0f,0x3a,0x0f,0x25,0xf0,0x1c,0xf0,0x1c,0x00]
palignr $0, 485498096, %mm4

// CHECK: palignr $0, 485498096, %xmm1
// CHECK: encoding: [0x66,0x0f,0x3a,0x0f,0x0d,0xf0,0x1c,0xf0,0x1c,0x00]
palignr $0, 485498096, %xmm1

// CHECK: palignr $0, 64(%edx,%eax), %mm4
// CHECK: encoding: [0x0f,0x3a,0x0f,0x64,0x02,0x40,0x00]
palignr $0, 64(%edx,%eax), %mm4

// CHECK: palignr $0, 64(%edx,%eax), %xmm1
// CHECK: encoding: [0x66,0x0f,0x3a,0x0f,0x4c,0x02,0x40,0x00]
palignr $0, 64(%edx,%eax), %xmm1

// CHECK: palignr $0, (%edx), %mm4
// CHECK: encoding: [0x0f,0x3a,0x0f,0x22,0x00]
palignr $0, (%edx), %mm4

// CHECK: palignr $0, (%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x3a,0x0f,0x0a,0x00]
palignr $0, (%edx), %xmm1

// CHECK: palignr $0, %mm4, %mm4
// CHECK: encoding: [0x0f,0x3a,0x0f,0xe4,0x00]
palignr $0, %mm4, %mm4

// CHECK: palignr $0, %xmm1, %xmm1
// CHECK: encoding: [0x66,0x0f,0x3a,0x0f,0xc9,0x00]
palignr $0, %xmm1, %xmm1

// CHECK: phaddd -485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x02,0xa4,0x82,0x10,0xe3,0x0f,0xe3]
phaddd -485498096(%edx,%eax,4), %mm4

// CHECK: phaddd 485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x02,0xa4,0x82,0xf0,0x1c,0xf0,0x1c]
phaddd 485498096(%edx,%eax,4), %mm4

// CHECK: phaddd -485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x02,0x8c,0x82,0x10,0xe3,0x0f,0xe3]
phaddd -485498096(%edx,%eax,4), %xmm1

// CHECK: phaddd 485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x02,0x8c,0x82,0xf0,0x1c,0xf0,0x1c]
phaddd 485498096(%edx,%eax,4), %xmm1

// CHECK: phaddd 485498096(%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x02,0xa2,0xf0,0x1c,0xf0,0x1c]
phaddd 485498096(%edx), %mm4

// CHECK: phaddd 485498096(%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x02,0x8a,0xf0,0x1c,0xf0,0x1c]
phaddd 485498096(%edx), %xmm1

// CHECK: phaddd 485498096, %mm4
// CHECK: encoding: [0x0f,0x38,0x02,0x25,0xf0,0x1c,0xf0,0x1c]
phaddd 485498096, %mm4

// CHECK: phaddd 485498096, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x02,0x0d,0xf0,0x1c,0xf0,0x1c]
phaddd 485498096, %xmm1

// CHECK: phaddd 64(%edx,%eax), %mm4
// CHECK: encoding: [0x0f,0x38,0x02,0x64,0x02,0x40]
phaddd 64(%edx,%eax), %mm4

// CHECK: phaddd 64(%edx,%eax), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x02,0x4c,0x02,0x40]
phaddd 64(%edx,%eax), %xmm1

// CHECK: phaddd (%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x02,0x22]
phaddd (%edx), %mm4

// CHECK: phaddd (%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x02,0x0a]
phaddd (%edx), %xmm1

// CHECK: phaddd %mm4, %mm4
// CHECK: encoding: [0x0f,0x38,0x02,0xe4]
phaddd %mm4, %mm4

// CHECK: phaddd %xmm1, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x02,0xc9]
phaddd %xmm1, %xmm1

// CHECK: phaddsw -485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x03,0xa4,0x82,0x10,0xe3,0x0f,0xe3]
phaddsw -485498096(%edx,%eax,4), %mm4

// CHECK: phaddsw 485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x03,0xa4,0x82,0xf0,0x1c,0xf0,0x1c]
phaddsw 485498096(%edx,%eax,4), %mm4

// CHECK: phaddsw -485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x03,0x8c,0x82,0x10,0xe3,0x0f,0xe3]
phaddsw -485498096(%edx,%eax,4), %xmm1

// CHECK: phaddsw 485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x03,0x8c,0x82,0xf0,0x1c,0xf0,0x1c]
phaddsw 485498096(%edx,%eax,4), %xmm1

// CHECK: phaddsw 485498096(%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x03,0xa2,0xf0,0x1c,0xf0,0x1c]
phaddsw 485498096(%edx), %mm4

// CHECK: phaddsw 485498096(%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x03,0x8a,0xf0,0x1c,0xf0,0x1c]
phaddsw 485498096(%edx), %xmm1

// CHECK: phaddsw 485498096, %mm4
// CHECK: encoding: [0x0f,0x38,0x03,0x25,0xf0,0x1c,0xf0,0x1c]
phaddsw 485498096, %mm4

// CHECK: phaddsw 485498096, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x03,0x0d,0xf0,0x1c,0xf0,0x1c]
phaddsw 485498096, %xmm1

// CHECK: phaddsw 64(%edx,%eax), %mm4
// CHECK: encoding: [0x0f,0x38,0x03,0x64,0x02,0x40]
phaddsw 64(%edx,%eax), %mm4

// CHECK: phaddsw 64(%edx,%eax), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x03,0x4c,0x02,0x40]
phaddsw 64(%edx,%eax), %xmm1

// CHECK: phaddsw (%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x03,0x22]
phaddsw (%edx), %mm4

// CHECK: phaddsw (%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x03,0x0a]
phaddsw (%edx), %xmm1

// CHECK: phaddsw %mm4, %mm4
// CHECK: encoding: [0x0f,0x38,0x03,0xe4]
phaddsw %mm4, %mm4

// CHECK: phaddsw %xmm1, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x03,0xc9]
phaddsw %xmm1, %xmm1

// CHECK: phaddw -485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x01,0xa4,0x82,0x10,0xe3,0x0f,0xe3]
phaddw -485498096(%edx,%eax,4), %mm4

// CHECK: phaddw 485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x01,0xa4,0x82,0xf0,0x1c,0xf0,0x1c]
phaddw 485498096(%edx,%eax,4), %mm4

// CHECK: phaddw -485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x01,0x8c,0x82,0x10,0xe3,0x0f,0xe3]
phaddw -485498096(%edx,%eax,4), %xmm1

// CHECK: phaddw 485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x01,0x8c,0x82,0xf0,0x1c,0xf0,0x1c]
phaddw 485498096(%edx,%eax,4), %xmm1

// CHECK: phaddw 485498096(%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x01,0xa2,0xf0,0x1c,0xf0,0x1c]
phaddw 485498096(%edx), %mm4

// CHECK: phaddw 485498096(%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x01,0x8a,0xf0,0x1c,0xf0,0x1c]
phaddw 485498096(%edx), %xmm1

// CHECK: phaddw 485498096, %mm4
// CHECK: encoding: [0x0f,0x38,0x01,0x25,0xf0,0x1c,0xf0,0x1c]
phaddw 485498096, %mm4

// CHECK: phaddw 485498096, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x01,0x0d,0xf0,0x1c,0xf0,0x1c]
phaddw 485498096, %xmm1

// CHECK: phaddw 64(%edx,%eax), %mm4
// CHECK: encoding: [0x0f,0x38,0x01,0x64,0x02,0x40]
phaddw 64(%edx,%eax), %mm4

// CHECK: phaddw 64(%edx,%eax), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x01,0x4c,0x02,0x40]
phaddw 64(%edx,%eax), %xmm1

// CHECK: phaddw (%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x01,0x22]
phaddw (%edx), %mm4

// CHECK: phaddw (%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x01,0x0a]
phaddw (%edx), %xmm1

// CHECK: phaddw %mm4, %mm4
// CHECK: encoding: [0x0f,0x38,0x01,0xe4]
phaddw %mm4, %mm4

// CHECK: phaddw %xmm1, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x01,0xc9]
phaddw %xmm1, %xmm1

// CHECK: phsubd -485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x06,0xa4,0x82,0x10,0xe3,0x0f,0xe3]
phsubd -485498096(%edx,%eax,4), %mm4

// CHECK: phsubd 485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x06,0xa4,0x82,0xf0,0x1c,0xf0,0x1c]
phsubd 485498096(%edx,%eax,4), %mm4

// CHECK: phsubd -485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x06,0x8c,0x82,0x10,0xe3,0x0f,0xe3]
phsubd -485498096(%edx,%eax,4), %xmm1

// CHECK: phsubd 485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x06,0x8c,0x82,0xf0,0x1c,0xf0,0x1c]
phsubd 485498096(%edx,%eax,4), %xmm1

// CHECK: phsubd 485498096(%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x06,0xa2,0xf0,0x1c,0xf0,0x1c]
phsubd 485498096(%edx), %mm4

// CHECK: phsubd 485498096(%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x06,0x8a,0xf0,0x1c,0xf0,0x1c]
phsubd 485498096(%edx), %xmm1

// CHECK: phsubd 485498096, %mm4
// CHECK: encoding: [0x0f,0x38,0x06,0x25,0xf0,0x1c,0xf0,0x1c]
phsubd 485498096, %mm4

// CHECK: phsubd 485498096, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x06,0x0d,0xf0,0x1c,0xf0,0x1c]
phsubd 485498096, %xmm1

// CHECK: phsubd 64(%edx,%eax), %mm4
// CHECK: encoding: [0x0f,0x38,0x06,0x64,0x02,0x40]
phsubd 64(%edx,%eax), %mm4

// CHECK: phsubd 64(%edx,%eax), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x06,0x4c,0x02,0x40]
phsubd 64(%edx,%eax), %xmm1

// CHECK: phsubd (%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x06,0x22]
phsubd (%edx), %mm4

// CHECK: phsubd (%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x06,0x0a]
phsubd (%edx), %xmm1

// CHECK: phsubd %mm4, %mm4
// CHECK: encoding: [0x0f,0x38,0x06,0xe4]
phsubd %mm4, %mm4

// CHECK: phsubd %xmm1, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x06,0xc9]
phsubd %xmm1, %xmm1

// CHECK: phsubsw -485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x07,0xa4,0x82,0x10,0xe3,0x0f,0xe3]
phsubsw -485498096(%edx,%eax,4), %mm4

// CHECK: phsubsw 485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x07,0xa4,0x82,0xf0,0x1c,0xf0,0x1c]
phsubsw 485498096(%edx,%eax,4), %mm4

// CHECK: phsubsw -485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x07,0x8c,0x82,0x10,0xe3,0x0f,0xe3]
phsubsw -485498096(%edx,%eax,4), %xmm1

// CHECK: phsubsw 485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x07,0x8c,0x82,0xf0,0x1c,0xf0,0x1c]
phsubsw 485498096(%edx,%eax,4), %xmm1

// CHECK: phsubsw 485498096(%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x07,0xa2,0xf0,0x1c,0xf0,0x1c]
phsubsw 485498096(%edx), %mm4

// CHECK: phsubsw 485498096(%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x07,0x8a,0xf0,0x1c,0xf0,0x1c]
phsubsw 485498096(%edx), %xmm1

// CHECK: phsubsw 485498096, %mm4
// CHECK: encoding: [0x0f,0x38,0x07,0x25,0xf0,0x1c,0xf0,0x1c]
phsubsw 485498096, %mm4

// CHECK: phsubsw 485498096, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x07,0x0d,0xf0,0x1c,0xf0,0x1c]
phsubsw 485498096, %xmm1

// CHECK: phsubsw 64(%edx,%eax), %mm4
// CHECK: encoding: [0x0f,0x38,0x07,0x64,0x02,0x40]
phsubsw 64(%edx,%eax), %mm4

// CHECK: phsubsw 64(%edx,%eax), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x07,0x4c,0x02,0x40]
phsubsw 64(%edx,%eax), %xmm1

// CHECK: phsubsw (%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x07,0x22]
phsubsw (%edx), %mm4

// CHECK: phsubsw (%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x07,0x0a]
phsubsw (%edx), %xmm1

// CHECK: phsubsw %mm4, %mm4
// CHECK: encoding: [0x0f,0x38,0x07,0xe4]
phsubsw %mm4, %mm4

// CHECK: phsubsw %xmm1, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x07,0xc9]
phsubsw %xmm1, %xmm1

// CHECK: phsubw -485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x05,0xa4,0x82,0x10,0xe3,0x0f,0xe3]
phsubw -485498096(%edx,%eax,4), %mm4

// CHECK: phsubw 485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x05,0xa4,0x82,0xf0,0x1c,0xf0,0x1c]
phsubw 485498096(%edx,%eax,4), %mm4

// CHECK: phsubw -485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x05,0x8c,0x82,0x10,0xe3,0x0f,0xe3]
phsubw -485498096(%edx,%eax,4), %xmm1

// CHECK: phsubw 485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x05,0x8c,0x82,0xf0,0x1c,0xf0,0x1c]
phsubw 485498096(%edx,%eax,4), %xmm1

// CHECK: phsubw 485498096(%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x05,0xa2,0xf0,0x1c,0xf0,0x1c]
phsubw 485498096(%edx), %mm4

// CHECK: phsubw 485498096(%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x05,0x8a,0xf0,0x1c,0xf0,0x1c]
phsubw 485498096(%edx), %xmm1

// CHECK: phsubw 485498096, %mm4
// CHECK: encoding: [0x0f,0x38,0x05,0x25,0xf0,0x1c,0xf0,0x1c]
phsubw 485498096, %mm4

// CHECK: phsubw 485498096, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x05,0x0d,0xf0,0x1c,0xf0,0x1c]
phsubw 485498096, %xmm1

// CHECK: phsubw 64(%edx,%eax), %mm4
// CHECK: encoding: [0x0f,0x38,0x05,0x64,0x02,0x40]
phsubw 64(%edx,%eax), %mm4

// CHECK: phsubw 64(%edx,%eax), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x05,0x4c,0x02,0x40]
phsubw 64(%edx,%eax), %xmm1

// CHECK: phsubw (%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x05,0x22]
phsubw (%edx), %mm4

// CHECK: phsubw (%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x05,0x0a]
phsubw (%edx), %xmm1

// CHECK: phsubw %mm4, %mm4
// CHECK: encoding: [0x0f,0x38,0x05,0xe4]
phsubw %mm4, %mm4

// CHECK: phsubw %xmm1, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x05,0xc9]
phsubw %xmm1, %xmm1

// CHECK: pmaddubsw -485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x04,0xa4,0x82,0x10,0xe3,0x0f,0xe3]
pmaddubsw -485498096(%edx,%eax,4), %mm4

// CHECK: pmaddubsw 485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x04,0xa4,0x82,0xf0,0x1c,0xf0,0x1c]
pmaddubsw 485498096(%edx,%eax,4), %mm4

// CHECK: pmaddubsw -485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x04,0x8c,0x82,0x10,0xe3,0x0f,0xe3]
pmaddubsw -485498096(%edx,%eax,4), %xmm1

// CHECK: pmaddubsw 485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x04,0x8c,0x82,0xf0,0x1c,0xf0,0x1c]
pmaddubsw 485498096(%edx,%eax,4), %xmm1

// CHECK: pmaddubsw 485498096(%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x04,0xa2,0xf0,0x1c,0xf0,0x1c]
pmaddubsw 485498096(%edx), %mm4

// CHECK: pmaddubsw 485498096(%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x04,0x8a,0xf0,0x1c,0xf0,0x1c]
pmaddubsw 485498096(%edx), %xmm1

// CHECK: pmaddubsw 485498096, %mm4
// CHECK: encoding: [0x0f,0x38,0x04,0x25,0xf0,0x1c,0xf0,0x1c]
pmaddubsw 485498096, %mm4

// CHECK: pmaddubsw 485498096, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x04,0x0d,0xf0,0x1c,0xf0,0x1c]
pmaddubsw 485498096, %xmm1

// CHECK: pmaddubsw 64(%edx,%eax), %mm4
// CHECK: encoding: [0x0f,0x38,0x04,0x64,0x02,0x40]
pmaddubsw 64(%edx,%eax), %mm4

// CHECK: pmaddubsw 64(%edx,%eax), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x04,0x4c,0x02,0x40]
pmaddubsw 64(%edx,%eax), %xmm1

// CHECK: pmaddubsw (%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x04,0x22]
pmaddubsw (%edx), %mm4

// CHECK: pmaddubsw (%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x04,0x0a]
pmaddubsw (%edx), %xmm1

// CHECK: pmaddubsw %mm4, %mm4
// CHECK: encoding: [0x0f,0x38,0x04,0xe4]
pmaddubsw %mm4, %mm4

// CHECK: pmaddubsw %xmm1, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x04,0xc9]
pmaddubsw %xmm1, %xmm1

// CHECK: pmulhrsw -485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x0b,0xa4,0x82,0x10,0xe3,0x0f,0xe3]
pmulhrsw -485498096(%edx,%eax,4), %mm4

// CHECK: pmulhrsw 485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x0b,0xa4,0x82,0xf0,0x1c,0xf0,0x1c]
pmulhrsw 485498096(%edx,%eax,4), %mm4

// CHECK: pmulhrsw -485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x0b,0x8c,0x82,0x10,0xe3,0x0f,0xe3]
pmulhrsw -485498096(%edx,%eax,4), %xmm1

// CHECK: pmulhrsw 485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x0b,0x8c,0x82,0xf0,0x1c,0xf0,0x1c]
pmulhrsw 485498096(%edx,%eax,4), %xmm1

// CHECK: pmulhrsw 485498096(%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x0b,0xa2,0xf0,0x1c,0xf0,0x1c]
pmulhrsw 485498096(%edx), %mm4

// CHECK: pmulhrsw 485498096(%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x0b,0x8a,0xf0,0x1c,0xf0,0x1c]
pmulhrsw 485498096(%edx), %xmm1

// CHECK: pmulhrsw 485498096, %mm4
// CHECK: encoding: [0x0f,0x38,0x0b,0x25,0xf0,0x1c,0xf0,0x1c]
pmulhrsw 485498096, %mm4

// CHECK: pmulhrsw 485498096, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x0b,0x0d,0xf0,0x1c,0xf0,0x1c]
pmulhrsw 485498096, %xmm1

// CHECK: pmulhrsw 64(%edx,%eax), %mm4
// CHECK: encoding: [0x0f,0x38,0x0b,0x64,0x02,0x40]
pmulhrsw 64(%edx,%eax), %mm4

// CHECK: pmulhrsw 64(%edx,%eax), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x0b,0x4c,0x02,0x40]
pmulhrsw 64(%edx,%eax), %xmm1

// CHECK: pmulhrsw (%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x0b,0x22]
pmulhrsw (%edx), %mm4

// CHECK: pmulhrsw (%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x0b,0x0a]
pmulhrsw (%edx), %xmm1

// CHECK: pmulhrsw %mm4, %mm4
// CHECK: encoding: [0x0f,0x38,0x0b,0xe4]
pmulhrsw %mm4, %mm4

// CHECK: pmulhrsw %xmm1, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x0b,0xc9]
pmulhrsw %xmm1, %xmm1

// CHECK: pshufb -485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x00,0xa4,0x82,0x10,0xe3,0x0f,0xe3]
pshufb -485498096(%edx,%eax,4), %mm4

// CHECK: pshufb 485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x00,0xa4,0x82,0xf0,0x1c,0xf0,0x1c]
pshufb 485498096(%edx,%eax,4), %mm4

// CHECK: pshufb -485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x00,0x8c,0x82,0x10,0xe3,0x0f,0xe3]
pshufb -485498096(%edx,%eax,4), %xmm1

// CHECK: pshufb 485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x00,0x8c,0x82,0xf0,0x1c,0xf0,0x1c]
pshufb 485498096(%edx,%eax,4), %xmm1

// CHECK: pshufb 485498096(%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x00,0xa2,0xf0,0x1c,0xf0,0x1c]
pshufb 485498096(%edx), %mm4

// CHECK: pshufb 485498096(%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x00,0x8a,0xf0,0x1c,0xf0,0x1c]
pshufb 485498096(%edx), %xmm1

// CHECK: pshufb 485498096, %mm4
// CHECK: encoding: [0x0f,0x38,0x00,0x25,0xf0,0x1c,0xf0,0x1c]
pshufb 485498096, %mm4

// CHECK: pshufb 485498096, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x00,0x0d,0xf0,0x1c,0xf0,0x1c]
pshufb 485498096, %xmm1

// CHECK: pshufb 64(%edx,%eax), %mm4
// CHECK: encoding: [0x0f,0x38,0x00,0x64,0x02,0x40]
pshufb 64(%edx,%eax), %mm4

// CHECK: pshufb 64(%edx,%eax), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x00,0x4c,0x02,0x40]
pshufb 64(%edx,%eax), %xmm1

// CHECK: pshufb (%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x00,0x22]
pshufb (%edx), %mm4

// CHECK: pshufb (%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x00,0x0a]
pshufb (%edx), %xmm1

// CHECK: pshufb %mm4, %mm4
// CHECK: encoding: [0x0f,0x38,0x00,0xe4]
pshufb %mm4, %mm4

// CHECK: pshufb %xmm1, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x00,0xc9]
pshufb %xmm1, %xmm1

// CHECK: psignb -485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x08,0xa4,0x82,0x10,0xe3,0x0f,0xe3]
psignb -485498096(%edx,%eax,4), %mm4

// CHECK: psignb 485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x08,0xa4,0x82,0xf0,0x1c,0xf0,0x1c]
psignb 485498096(%edx,%eax,4), %mm4

// CHECK: psignb -485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x08,0x8c,0x82,0x10,0xe3,0x0f,0xe3]
psignb -485498096(%edx,%eax,4), %xmm1

// CHECK: psignb 485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x08,0x8c,0x82,0xf0,0x1c,0xf0,0x1c]
psignb 485498096(%edx,%eax,4), %xmm1

// CHECK: psignb 485498096(%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x08,0xa2,0xf0,0x1c,0xf0,0x1c]
psignb 485498096(%edx), %mm4

// CHECK: psignb 485498096(%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x08,0x8a,0xf0,0x1c,0xf0,0x1c]
psignb 485498096(%edx), %xmm1

// CHECK: psignb 485498096, %mm4
// CHECK: encoding: [0x0f,0x38,0x08,0x25,0xf0,0x1c,0xf0,0x1c]
psignb 485498096, %mm4

// CHECK: psignb 485498096, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x08,0x0d,0xf0,0x1c,0xf0,0x1c]
psignb 485498096, %xmm1

// CHECK: psignb 64(%edx,%eax), %mm4
// CHECK: encoding: [0x0f,0x38,0x08,0x64,0x02,0x40]
psignb 64(%edx,%eax), %mm4

// CHECK: psignb 64(%edx,%eax), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x08,0x4c,0x02,0x40]
psignb 64(%edx,%eax), %xmm1

// CHECK: psignb (%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x08,0x22]
psignb (%edx), %mm4

// CHECK: psignb (%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x08,0x0a]
psignb (%edx), %xmm1

// CHECK: psignb %mm4, %mm4
// CHECK: encoding: [0x0f,0x38,0x08,0xe4]
psignb %mm4, %mm4

// CHECK: psignb %xmm1, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x08,0xc9]
psignb %xmm1, %xmm1

// CHECK: psignd -485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x0a,0xa4,0x82,0x10,0xe3,0x0f,0xe3]
psignd -485498096(%edx,%eax,4), %mm4

// CHECK: psignd 485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x0a,0xa4,0x82,0xf0,0x1c,0xf0,0x1c]
psignd 485498096(%edx,%eax,4), %mm4

// CHECK: psignd -485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x0a,0x8c,0x82,0x10,0xe3,0x0f,0xe3]
psignd -485498096(%edx,%eax,4), %xmm1

// CHECK: psignd 485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x0a,0x8c,0x82,0xf0,0x1c,0xf0,0x1c]
psignd 485498096(%edx,%eax,4), %xmm1

// CHECK: psignd 485498096(%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x0a,0xa2,0xf0,0x1c,0xf0,0x1c]
psignd 485498096(%edx), %mm4

// CHECK: psignd 485498096(%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x0a,0x8a,0xf0,0x1c,0xf0,0x1c]
psignd 485498096(%edx), %xmm1

// CHECK: psignd 485498096, %mm4
// CHECK: encoding: [0x0f,0x38,0x0a,0x25,0xf0,0x1c,0xf0,0x1c]
psignd 485498096, %mm4

// CHECK: psignd 485498096, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x0a,0x0d,0xf0,0x1c,0xf0,0x1c]
psignd 485498096, %xmm1

// CHECK: psignd 64(%edx,%eax), %mm4
// CHECK: encoding: [0x0f,0x38,0x0a,0x64,0x02,0x40]
psignd 64(%edx,%eax), %mm4

// CHECK: psignd 64(%edx,%eax), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x0a,0x4c,0x02,0x40]
psignd 64(%edx,%eax), %xmm1

// CHECK: psignd (%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x0a,0x22]
psignd (%edx), %mm4

// CHECK: psignd (%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x0a,0x0a]
psignd (%edx), %xmm1

// CHECK: psignd %mm4, %mm4
// CHECK: encoding: [0x0f,0x38,0x0a,0xe4]
psignd %mm4, %mm4

// CHECK: psignd %xmm1, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x0a,0xc9]
psignd %xmm1, %xmm1

// CHECK: psignw -485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x09,0xa4,0x82,0x10,0xe3,0x0f,0xe3]
psignw -485498096(%edx,%eax,4), %mm4

// CHECK: psignw 485498096(%edx,%eax,4), %mm4
// CHECK: encoding: [0x0f,0x38,0x09,0xa4,0x82,0xf0,0x1c,0xf0,0x1c]
psignw 485498096(%edx,%eax,4), %mm4

// CHECK: psignw -485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x09,0x8c,0x82,0x10,0xe3,0x0f,0xe3]
psignw -485498096(%edx,%eax,4), %xmm1

// CHECK: psignw 485498096(%edx,%eax,4), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x09,0x8c,0x82,0xf0,0x1c,0xf0,0x1c]
psignw 485498096(%edx,%eax,4), %xmm1

// CHECK: psignw 485498096(%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x09,0xa2,0xf0,0x1c,0xf0,0x1c]
psignw 485498096(%edx), %mm4

// CHECK: psignw 485498096(%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x09,0x8a,0xf0,0x1c,0xf0,0x1c]
psignw 485498096(%edx), %xmm1

// CHECK: psignw 485498096, %mm4
// CHECK: encoding: [0x0f,0x38,0x09,0x25,0xf0,0x1c,0xf0,0x1c]
psignw 485498096, %mm4

// CHECK: psignw 485498096, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x09,0x0d,0xf0,0x1c,0xf0,0x1c]
psignw 485498096, %xmm1

// CHECK: psignw 64(%edx,%eax), %mm4
// CHECK: encoding: [0x0f,0x38,0x09,0x64,0x02,0x40]
psignw 64(%edx,%eax), %mm4

// CHECK: psignw 64(%edx,%eax), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x09,0x4c,0x02,0x40]
psignw 64(%edx,%eax), %xmm1

// CHECK: psignw (%edx), %mm4
// CHECK: encoding: [0x0f,0x38,0x09,0x22]
psignw (%edx), %mm4

// CHECK: psignw (%edx), %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x09,0x0a]
psignw (%edx), %xmm1

// CHECK: psignw %mm4, %mm4
// CHECK: encoding: [0x0f,0x38,0x09,0xe4]
psignw %mm4, %mm4

// CHECK: psignw %xmm1, %xmm1
// CHECK: encoding: [0x66,0x0f,0x38,0x09,0xc9]
psignw %xmm1, %xmm1
