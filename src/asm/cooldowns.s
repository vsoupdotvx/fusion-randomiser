.section .text

"CardUI::Awake(&mut self)+0xDC":
	call store_cooldown
"ENDCardUI::Awake(&mut self)+0xDC":

store_cooldown:
	pushq  %rax
	
	movss CardUI.fullCD(%rbx), %xmm1
	movl CardUI.theSeedType(%rbx), %ecx
	cmpl $1181, %ecx
	ja   store_cooldown.locA
		call plant_type_flatten_menu
		cmpl $48, %eax
		jnc  store_cooldown.locA
			leaq     plant_cd_table(%rip), %rdx
			movzbl   (%rdx,%rax),     %edx
			xorl     %ecx,            %ecx
			shlb     $1,               %dl
			setnc    %cl
			cvtsi2ss %edx,           %xmm0
			mulss    const1over254(%rip), %xmm0
			addss    const1.0(%rip), %xmm0
			jrcxz    store_cooldown.locB
				mulss const0.5(%rip), %xmm0
			store_cooldown.locB:
			mulss %xmm0, %xmm1
	store_cooldown.locA:
	
	movss %xmm1, CardUI.fullCD(%rbx)
	movss CardUI.CD(%rbx), %xmm1
	popq  %rax
	ret

fetch_cooldown:
	call   plant_type_flatten_menu
	testq  %rax, %rax
	js     fetch_cooldown.locA
		leaq   plant_cd_table(%rip), %rcx
		movzbl (%rcx,%rax),       %eax
	fetch_cooldown.locA:
	ret

.section .data
const1.0:
	.float 1.0
const0.5:
	.float 0.5
const1over254:
	.float 0.00393700787402
plant_cd_table:
	.space 48
