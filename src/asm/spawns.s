.section .text

"InitZombieList::AdvantureZombieTypeSpawn(theLevelNumber: i32)+0x30":
	.nops 0x15
"ENDInitZombieList::AdvantureZombieTypeSpawn(theLevelNumber: i32)+0x30":

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

.section .data
zombie_spawn_bitfield: #size: 46 + 19 + 24 = 89
	.quad 0xFFFFAFFFFFFFFFFF
	.quad 0x1FFFFFF
zombie_weights:
	.space 512, 0x0
