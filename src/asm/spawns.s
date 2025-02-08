.section .text

"InitZombieList::AdvantureZombieTypeSpawn(theLevelNumber: i32)+0x30":
	.nops 0x15
"ENDInitZombieList::AdvantureZombieTypeSpawn(theLevelNumber: i32)+0x30":

"InitZombieList::AdvantureZombieTypeSpawn(theLevelNumber: i32)+0x82":
	jmp init_zombie_list
"ENDInitZombieList::AdvantureZombieTypeSpawn(theLevelNumber: i32)+0x82":
nop #the assembled object file has no distinction between if the last jump is
#trying to branch inside or outside the patch and choses inside if its a jmp
#instruction, which is wrong. this nop corrects that
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

.section .data
zombie_spawn_bitfield: #size: 46 + 19 + 24 = 89
	.quad 0xFFFFAFFFFFFFFFFF
	.quad 0x1FFFFFF
zombie_weights:
	.space 512, 0x0
