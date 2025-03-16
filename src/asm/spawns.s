.section .text

"InitZombieList::AdvantureZombieTypeSpawn(theLevelNumber: i32)+0x43":
	.nops 2
"ENDInitZombieList::AdvantureZombieTypeSpawn(theLevelNumber: i32)+0x43":

"InitZombieList::AdvantureZombieTypeSpawn(theLevelNumber: i32)+0x82":
	jmp init_zombie_list
"ENDInitZombieList::AdvantureZombieTypeSpawn(theLevelNumber: i32)+0x82":

"InitZombieList::PickZombie() -> ZombieType":
	jmp pick_zombie
"ENDInitZombieList::PickZombie() -> ZombieType":

insb

init_zombie_list:
	movq %rcx, %rbx
	
	movq zombie_spawn_bitfield(%rip), %rdx
	init_zombie_list.loopA:
		bsfq %rdx, %rcx
		je init_zombie_list.exloopA
			btrq %rcx,          %rdx
			call zombie_type_widen
			cmpl %eax,    0x18(%rbx)
			jna  init_zombie_list.exloopA
			movb $1, 0x20(%rbx,%rax)
	jmp init_zombie_list.loopA
	init_zombie_list.exloopA:
	
	movq zombie_spawn_bitfield+0x8(%rip), %rdx
	init_zombie_list.loopB:
		bsfq %rdx, %rcx
		je init_zombie_list.exloopB
			btrq %rcx,          %rdx
			addl $0x40,         %ecx
			call zombie_type_widen
			cmpl %eax,    0x18(%rbx)
			jna  init_zombie_list.exloopB
			movb $1, 0x20(%rbx,%rax)
	jmp init_zombie_list.loopB
	init_zombie_list.exloopB:
	
	addq $0x20, %rsp
	popq %rbx
	ret

pick_zombie:
	pushq %rbp
	pushq %rbx
	movq  %rsp, %rbp
	
	leaq zombie_spawn_bitfield(%rip), %r8
	xorl %edx, %edx
	movl $127, %eax
	pick_zombie.loopA:
		movq %rax,         %r9
		movl %eax,        %ecx
		andq $0x3F,        %r9
		shrl $6,          %ecx
		btq  %r9, (%r8,%rcx,8)
		jnc pick_zombie.locA
			addl  16(%r8,%rax,4), %edx
			pushq %rax
		pick_zombie.locA:
		subl $1, %eax
	jnc pick_zombie.loopA
	
	movq %rsp,  %rbx
	xorl %ecx,  %ecx
	andq $-16,  %rsp
	subq $0x20, %rsp
	call "UnityEngine::Random::RandomRangeInt(minInclusive: i32, maxExclusive: i32) -> i32"
	movq %rbx,  %rsp
	
	leaq zombie_weights(%rip), %rdx
	pick_zombie.loopB:
		popq %rcx
		subl (%rdx,%rcx,4), %eax
	jnc pick_zombie.loopB
	
	call zombie_type_widen
	
	movq %rbp, %rsp
	popq %rbx
	popq %rbp
	ret

"Zombie::InitHealth(&mut self)+0x18B":
	call  show_text_if_preview_1
	.nops 2
"ENDZombie::InitHealth(&mut self)+0x18B":

"CreateZombie::SetZombie(&mut self, theRow: i32, theZombieType: ZombieType, theX: f32, isIdle: bool) -> UnityEngine::GameObject+0x5BC":
	jne show_text_if_preview_2
"ENDCreateZombie::SetZombie(&mut self, theRow: i32, theZombieType: ZombieType, theX: f32, isIdle: bool) -> UnityEngine::GameObject+0x5BC":

"Zombie::UpdateHealthText(&mut self)+0x8E":
	call set_zombie_txt
"ENDZombie::UpdateHealthText(&mut self)+0x8E":

show_text_if_preview_1:
	movzbl Board.showZombieHealth(%rdx), %edx
	orb    on_seed_select(%rip), %dl
	ret

show_text_if_preview_2:
	movq %rdi, %rcx
	xorl %edx, %edx
	call "Zombie::InitHealth(&mut self)"
	jmp  "CreateZombie::SetZombie(&mut self, theRow: i32, theZombieType: ZombieType, theX: f32, isIdle: bool) -> UnityEngine::GameObject"+0x82D

set_zombie_txt:
	cmpb $0, on_seed_select(%rip)
	jne  set_zombie_txt.locA
	xorl %edx, %edx
	xorq %r8,   %r8
	ret
	set_zombie_txt.locA:
	subq $0x48, %rsp
	movl $66, %ebx
	
	movl  Zombie.theZombieType(%rdi), %ecx
	leaq  zombie_weights(%rip), %rdx
	call  zombie_type_flatten
	movl  (%rdx,%rax,4),     %ecx
	movq  %rcx,        0x40(%rsp)
	movss 0x200(%rdx,%rax), %xmm0
	leaq  0x30(%rsp),        %rcx
	call  float_2_string_4sf_0t3xp
	movq  0x40(%rsp),        %rcx
	leaq  0x20(%rsp),        %rdx
	call  int_2_string_fast
	addl  %eax,              %ebx
	movl  %eax,              %ebp
	
	movl   %ebx, %ecx
	shrl   $1,   %ecx
	call   "System::String::FastAllocateString(length: i32) -> String"
	movq   %rdi, 0x40(%rsp)
	movq   %rax, %rdi
	xorl   %ecx, %ecx
	call   "System.Runtime.CompilerServices::RuntimeHelpers::get_OffsetToStringData() -> i32"
	pushq  %rdi
	movslq %eax,  %r8
	addq   %r8,  %rdi
	
	movq  $0x0067006900650057, %rax
	stosq
	movq  $0x0020003A00740068, %rax
	stosq
	
	pushq %rsi
	leaq  0x40(%rsp), %rsi
	movl  %ebp,       %ecx
	subq  %rbp,       %rsi
	
	rep movsb
	
	leaq average_txt(%rip), %rsi
	movb $20,                %cl
	rep  movsw
	
	movq 0x40(%rsp), %rax
	stosq
	movl 0x48(%rsp), %eax
	stosw
	
	popq %rsi
	popq %rax
	movq 0x40(%rsp), %rdi
	addq $0x50,      %rsp
	
	movq %rsi, 0x38(%rsp)
	movq %r14, 0x30(%rsp)
	movq Zombie.healthText(%rdi), %r14
	jmp  "Zombie::UpdateHealthText(&mut self)"+0x337

int_2_string_fast:
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
	
	palignr $12, %xmm1, %xmm0
	palignr $12, %xmm2, %xmm1
	subps   %xmm0,      %xmm5
	subps   %xmm2,      %xmm4
	
	cvtps2dq %xmm5, %xmm5
	cvtps2dq %xmm4, %xmm4
	
	packssdw %xmm5, %xmm4
	
	pcmpeqw  %xmm4,         %xmm2
	pmovmskb %xmm2,          %eax
	xorl     $0xFFFF,        %eax
	orl      $0xC000,        %eax
	bsfl     %eax,           %eax
	negl     %eax
	addl     $16,            %eax
	
	paddw  const8x0x30(%rip), %xmm4
	movdqu %xmm4,            (%rdx)
	ret

float_2_string_4sf_0t3xp: #2 sigfigs, 0-3 (decimal) exponent
	xorl    %edx,              %edx
	ucomiss const100.0(%rip), %xmm0
	jc float_2_string_4sf_0t3xp.locA
		ucomiss const1000.0(%rip), %xmm0
		setnc   %dl
		addb    $2,                  %dl
	jmp float_2_string_4sf_0t3xp.locB
	float_2_string_4sf_0t3xp.locA:
		ucomiss const10.0(%rip), %xmm0
		setnc   %dl
	float_2_string_4sf_0t3xp.locB:
	
	leaq   float_2_string_4sf_0t3xp.lutA(%rip), %rax
	movdqa const4x10.0(%rip), %xmm2
	movdqa float_2_string_4sf_0t3xp.constA(%rip), %xmm3
	cmpb   $3,            %dl
	jne    float_2_string_4sf_0t3xp.locC
		movdqa float_2_string_4sf_0t3xp.constB(%rip), %xmm3
	float_2_string_4sf_0t3xp.locC:
	mulss    (%rax,%rdx,4),     %xmm0
	leaq     float_2_string_4sf_0t3xp.lutB(%rip), %rax
	shll     $4,                 %edx
	addss    const0.5(%rip),    %xmm0
	pshufd   $0x00,   %xmm0,    %xmm1
	mulss    card_create_label.constB(%rip), %xmm1
	roundps  $3,     %xmm1,     %xmm1
	mulps    %xmm1,             %xmm2
	psrldq   $4,                %xmm2
	subps    %xmm2,             %xmm1
	cvtps2dq %xmm1,             %xmm1
	paddd    const4x0x30(%rip), %xmm1
	packssdw %xmm3,             %xmm1
	pshufb   (%rax,%rdx),       %xmm1
	movdqa   %xmm1,            (%rcx)
	ret

.section .data
const4x0x30:
	.long 0x30
	.long 0x30
	.long 0x30
	.long 0x30
float_2_string_4sf_0t3xp.constA:
	.long 0
	.long 0
	.long 0
	.long 0x2E
float_2_string_4sf_0t3xp.constB:
	.long 0
	.long 0
	.long 0
	.long 0x2C
float_2_string_4sf_0t3xp.lutA:
	const1000.0:
		.float 1000.0
	const100.0:
		.float 100.0
	const10.0:
		.float 10.0
	.float 1.0
float_2_string_4sf_0t3xp.lutB:
	.word  0x0100
	.word  0x0F0E
	.word  0x0302
	.word  0x0504
	.word  0x0706
	.space 0x6, 0x8
	.word  0x0100
	.word  0x0302
	.word  0x0F0E
	.word  0x0504
	.word  0x0706
	.space 0x6, 0x8
	.word  0x0100
	.word  0x0302
	.word  0x0504
	.word  0x0F0E
	.word  0x0706
	.space 0x6, 0x8
	.word  0x0100
	.word  0x0F0E
	.word  0x0302
	.word  0x0504
	.word  0x0706
	.space 0x6, 0x8
const0.5:
	.float 0.5
average_txt:
	.ascii "\n\0A\0v\0e\0r\0a\0g\0e\0 \0f\0r\0e\0q\0u\0e\0n\0c\0y\0:\0 \0"
.align 16
zombie_spawn_bitfield: #size: 46 + 19 + 24 = 89
	.quad 0xFFFFAFFFFFFFFFFF
	.quad 0x7FFFFFF
zombie_weights:
	.space 512, 0x0
zombie_freqs:
	.space 512, 0x0
