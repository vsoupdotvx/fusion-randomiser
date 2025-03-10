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
	subq $0x38, %rsp
	movl $16, %ebx
	
	movl Zombie.theZombieType(%rdi), %ecx
	leaq zombie_weights(%rip), %rdx
	call zombie_type_flatten
	movl (%rdx,%rax,4), %ecx
	leaq 0x20(%rsp),    %rdx
	call int_2_string_fast
	addl %eax,          %ebx
	movl %eax,          %ebp
	
	movl   %ebx, %ecx
	shrl   $1,   %ecx
	call   "System::String::FastAllocateString(length: i32) -> String"
	movq   %rdi, 0x30(%rsp)
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
	
	popq %rsi
	popq %rax
	movq 0x30(%rsp), %rdi
	addq $0x40,      %rsp
	
	movq %rsi, 0x38(%rsp)
	movq %r14, 0x30(%rsp)
	movq Zombie.healthText(%rdi), %r14
	jmp "Zombie::UpdateHealthText(&mut self)"+0x337

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
	

.section .data
zombie_spawn_bitfield: #size: 46 + 19 + 24 = 89
	.quad 0xFFFFAFFFFFFFFFFF
	.quad 0x7FFFFFF
zombie_weights:
	.space 512, 0x0
