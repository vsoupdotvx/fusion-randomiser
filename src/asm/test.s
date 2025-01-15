.section .text

adventure_level_enter_2:
	leaq   level_lut(%rip), %rcx
	movzbl (%rcx,%rdx),     %edx
	xorq   %r8,              %r8
	#hlt
	#insb
	hlt
	movl   InGameUIMgr.ShowZombieHealth(%rbx), %ecx
	#insb
	ret
things_to_test:
#	subl   $a+1,       b(%rax)
#	movl   $0,         c(%rax)
	insb
	addl $1, level_lut(%rip)
	call adventure_level_enter
	insb
	jmp  adventure_level_enter
	#insb
	jne  adventure_level_enter
#	movabs $d,            %rax
#	movabs $plant_lut,    %rax
#	addl   $3, plant_lut(%rip)
