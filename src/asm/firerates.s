.section .text

.macro .utf8 char #not actually used because utf16
	.ifeq \char >> 6
		.byte \char
	.else
		.ifeq \char >> 11
			.byte 0xC0 | \char >> 6
			.byte 0x80 | (\char & 0x3F)
		.else
			.ifeq \char >> 16
				.byte 0xE0 | \char >> 12
				.byte 0x80 | (\char >> 6 & 0x3F)
				.byte 0x80 | (\char & 0x3F)
			.else
				.byte 0xF0 | \char >> 18
				.byte 0x80 | (\char >> 12 & 0x3F)
				.byte 0x80 | (\char >> 6 & 0x3F)
				.byte 0x80 | (\char & 0x3F)
			.endif
		.endif
	.endif
.endm

"Plant::PlantShootUpdate(&mut self)+0x5A":
	call plant_get_firerate
"ENDPlant::PlantShootUpdate(&mut self)+0x5A":

plant_get_firerate:
	movl Plant.thePlantType(%rbx), %edx
	cmpl $1192, %edx #MAX_PLANT
	ja   plant_get_firerate.locA
		call     plant_type_flatten
		
		leaq     plant_firerate_table(%rip), %rdx
		movzbq   (%rdx,%rax), %rdx
		shlb     $1,           %dl
		cvtsi2ss %edx,       %xmm6
		mulss    const1over254(%rip), %xmm6
		addss    const1.0(%rip), %xmm6
		jc       plant_get_firerate.locB
			mulss const0.5(%rip), %xmm6
	jmp plant_get_firerate.locB
	plant_get_firerate.locA:
		movss const1.0(%rip), %xmm6
	plant_get_firerate.locB:
	mulss Plant.thePlantAttackInterval(%rbx), %xmm6
	ret

fetch_firerate:
	call   plant_type_flatten
	leaq   plant_firerate_table(%rip), %rcx
	movzbl (%rcx,%rax),  %eax
	ret

.section .data
card_create_label.constA:
	.float 0.0000001
	.float 0.000001
	.float 0.00001
	.float 0.0001
card_create_label.constB:
	.float 0.001
	.float 0.01
	.float 0.1
	.float 1
const8x0x30:
	.word 0x30
	.word 0x30
	.word 0x30
	.word 0x30
	.word 0x30
	.word 0x30
	.word 0x30
	.word 0x30
const4x10.0:
	.float 10
	.float 10
	.float 10
	.float 10
const1.0:
	.float 1.0
const0.5:
	.float 0.5
indicator_lut:
	.word 0x0039; .word 0x0020; .word 0x007C; .word 0x0020
	.word 0x0038; .word 0x0020; .word 0x007C; .word 0x0020
	.word 0x0037; .word 0x0020; .word 0x007C; .word 0x0020
	.word 0x0036; .word 0x0020; .word 0x007C; .word 0x0020
	.word 0x0035; .word 0x0020; .word 0x007C; .word 0x0020
	.word 0x0034; .word 0x0020; .word 0x007C; .word 0x0020
	.word 0x0033; .word 0x0020; .word 0x007C; .word 0x0020
	.word 0x0032; .word 0x0020; .word 0x007C; .word 0x0020
	.word 0x0031; .word 0x0020; .word 0x007C; .word 0x0020
	.word 0x0030; .word 0x0020; .word 0x007C; .word 0x0020
const1over254:
	.float 0.00393700787402
plant_firerate_table:
	.space 384, 0x00
plant_firerate_table_end:
