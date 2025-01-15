.section .text

"InGameUIMgr::UnlockCard(&mut self, theSeedType: i32) -> bool+0xb1": #0x58 bytes
	leaq   plant_lut(%rip), %rax #7
	movzbl (%rax,%rbp),     %eax #4
	cmpl   %ebp,      0x20(%rsi) #3
	hlt
	jne    "InGameUIMgr::UnlockCard.locE" #6
	movq   %rsi,            %rcx #3
	.nops  0x41
"ENDInGameUIMgr::UnlockCard(&mut self, theSeedType: i32) -> bool+0xb1":

"Advanture_Btn::OnMouseUp(&mut self)+0x4e":
	call adventure_level_enter
	nop
"ENDAdvanture_Btn::OnMouseUp(&mut self)+0x4e":

adventure_level_enter:
	leaq   level_lut(%rip), %rcx
	movsbq (%rcx,%rdx),     %rdx
	xorq   %r8,              %r8
	movl   Advanture_Btn.levelType(%rbx), %ecx
	ret

MAX_PLANT = 1160
plant_type_flatten: #This likely needs to be checked every single update
	xorl %eax,  %eax
	
	cmpl $1000, %ecx
	jc   plant_type_flatten.locA
		subl $75, %ecx
	plant_type_flatten.locA:
	
	testl %ecx, %ecx
	sets  %al
	
	cmpl  $900, %ecx
	jc    plant_type_flatten.locB
		subl $643, %ecx
	plant_type_flatten.locB:
	
	negq %rax
	
	cmpl $245, %ecx
	jc   plant_type_flatten.locC
		subl $212, %ecx
	plant_type_flatten.locC:
	
	orq %rcx, %rax
	
	ret

.section .data
level_lut:
	.space 50, 0x2
plant_lut:
	.space 50
