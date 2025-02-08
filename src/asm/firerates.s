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

"CardUI::Awake(&mut self)+0x24F":
	call card_create_label
"ENDCardUI::Awake(&mut self)+0x24F":

"CardUI::Update(&mut self)+0x108":
	call card_create_label
"ENDCardUI::Update(&mut self)+0x108":

"Plant::PlantShootUpdate(&mut self)+0x5A":
	call plant_get_firerate
"ENDPlant::PlantShootUpdate(&mut self)+0x5A":

plant_get_firerate:
	movl Plant.thePlantType(%rbx), %edx
	cmpl $1178, %edx #MAX_PLANT
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

card_create_label: #seed packet cost in ecx
	pushq %rdi
	pushq %rsi
	pushq %rbp
	pushq %rbx
	pushq %r15
	subq  $0x40,  %rsp
	xorl  %r15d, %r15d
	movl  (%rcx), %ecx
	
	testl %ecx,   %ecx
	sets  %r15b
	jns   card_create_label.locA
		negl %ecx
	card_create_label.locA:
	
	pxor     %xmm2,             %xmm2
	movaps   const4x10.0(%rip), %xmm0
	cvtsi2ss %ecx,              %xmm5
	pshufd   $0,     %xmm5,     %xmm4
	pshufd   $0,     %xmm5,     %xmm5
	
	mulps  card_create_label.constB(%rip), %xmm5
	mulps  card_create_label.constA(%rip), %xmm4
	movaps %xmm0, %xmm1
	
	roundps $3, %xmm5, %xmm5
	roundps $3, %xmm4, %xmm4
	mulps   %xmm5,     %xmm0
	mulps   %xmm4,     %xmm1
	
	palignr $12, %xmm0, %xmm1
	palignr $12, %xmm2, %xmm0
	subps   %xmm0,      %xmm5
	subps   %xmm2,      %xmm4
	
	cvtps2dq %xmm5, %xmm5
	cvtps2dq %xmm4, %xmm4
	
	packssdw %xmm5, %xmm4
	
	pcmpeqw  %xmm4,          %xmm2
	pmovmskb %xmm2,           %esi
	xorl     $0xFFFF,         %esi
	orl      $0xC000,         %esi
	bsfl     %esi,            %esi
	negl     %esi
	addl     $16,             %esi
	leal     8(%esi,%r15d,2), %ecx
	shrl     $1,              %ecx
	
	paddw  const8x0x30(%rip), %xmm4
	movdqu %xmm4, 0x20(%rsp)
	
	xorl %edx, %edx
	call "System::String::FastAllocateString(length: i32) -> String"
	movq %rax, %rbp
	xorl %ecx, %ecx
	call "System.Runtime.CompilerServices::RuntimeHelpers::get_OffsetToStringData() -> i32"
	
	testl %r15d, %r15d
	je    card_create_label.locB
	    movw $0x002D, 8(%rbp,%rax)
	card_create_label.locB:
	
	movslq %eax, %r8
	addq   %r8, %rbp
	
	movslq CardUI.theSeedType(%rbx), %rcx
	movl   $0x80, %ebx
	cmpl   $1160, %ecx
	ja     card_create_label.locC
		call plant_type_flatten #doesn't affect %r8
		leaq plant_firerate_table(%rip), %rdx
		movb (%rdx,%rax), %bl
	card_create_label.locC:
	
	movl %esi,           %ecx
	leaq 0x30(%rsp),     %rsi
	leaq 8(%rbp,%r15,2), %rdi
	subq %rcx,           %rsi
	shrl $1,             %ecx
	
	rep movsw #slower than rep movsb on fast small rep movsb systems
	
	imull $9,   %ebx,   %ebx
	shrl  $8,           %ebx
	leaq indicator_lut(%rip), %rcx
	movq (%rcx,%rbx,8), %rcx
	movq %rcx,        (%rbp)
	
	addq $0x40,       %rsp
	movq %rbp,        %rax
	subq %r8,         %rax
	popq %r15
	popq %rbx
	popq %rbp
	popq %rsi
	popq %rdi
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
