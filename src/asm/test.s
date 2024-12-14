.section .text
#"il2cppfunction1+0x38":
#    nop
#    il2cppfunction1.locA:
#        call il2cppfunction2
#        movl il2cppstructoffset(%rax), %eax
#        
#    jmp  il2cppfunction1.locA
#    nop
"InGameUIMgr::UnlockCard(&mut self, theSeedType: i32) -> bool+0xb1": #0x58 bytes
	movq   $order_lut,  %rax #10
	movzbl (%rax,%rbp), %eax #3
	cmpl   %ebp,  0x20(%rsi) #3
	jne    "InGameUIMgr::UnlockCard.locE" #2
	movq   %rsi,        %rcx #3
	.nops  0x43
	