.section .text

"CardUI::Awake(&mut self)+0x173":
	call cmp_and_store_cost
	nop
"ENDCardUI::Awake(&mut self)+0x173":

cmp_and_store_cost:
	pushq %rdx
	movl  %eax, %edx
	movl  CardUI.theSeedType(%rbx), %ecx
	cmpl  $1227, %ecx
	ja    cmp_and_store_cost.locA
		call plant_type_flatten_menu
		cmpl $48, %eax
		jnc  cmp_and_store_cost.locA
			leaq     plant_cost_table(%rip), %rcx
			movzbl   (%rcx,%rax),     %ecx
			shlb     $1,               %cl
			cvtsi2ss %ecx,           %xmm2
			cvtsi2ss %edx,           %xmm3
			mulss    const0.2over254(%rip), %xmm2
			addss    const0.2(%rip), %xmm2
			jc       cmp_and_store_cost.locB
				mulss const0.5(%rip), %xmm2
			cmp_and_store_cost.locB:
			mulss    %xmm3,        %xmm2
			cvtss2si %xmm2,         %edx
			leal     (%edx,%edx,4), %edx
	cmp_and_store_cost.locA:
	
	movl    %edx, CardUI.theSeedCost(%rbx)
	movl    %edx,   %eax
	ucomiss %xmm0, %xmm1
	popq    %rdx
	ret

.section .data
const0.2:
	.float 0.20001 #fixes some rounding issues I was having
const0.5:
	.float 0.5
const5.0:
	.float 5.0
const0.2over254:
	.float 0.000787401574804
plant_cost_table:
	.space 48
