.section .text

"GameAPP::PlaySound(theSoundID: i32, theVolume: f32, pitch: f32)+0x254":
	call sound_replace_A
"ENDGameAPP::PlaySound(theSoundID: i32, theVolume: f32, pitch: f32)+0x254":

sound_replace_A:
	pushq %rdx
	    
	leaq 0x20(%rcx), %r8
	movl $0x87,     %ecx #total number of sounds in GameAPP::LoadSound()
	movl %edi,      %edx
	call sound_replace
	movq %rax,      %r14
	
	popq %rdx
	ret

sound_replace:
	movl  %edx,               %eax
	xorl  rng_seed_1(%rip),   %edx
	imull $0x27220A95,  %edx, %edx #taken from https://docs.rs/fxhash/latest/src/fxhash/lib.rs.html
	cmpq  sound_chance(%rip), %rdx
	ja    sound_replace.locA
		xorl  rng_seed_2(%rip),  %eax
		imull $0x27220A95, %eax, %eax
		imulq %rcx,              %rax
		shrq  $32,               %rax
	sound_replace.locA:
	movq (%r8,%rax,8), %rax
	ret

.section .data
sound_rng_seed:
	rng_seed_1: .long 0x01234567
	rng_seed_2: .long 0x89ABCDEF

sound_chance:
	.quad 0
