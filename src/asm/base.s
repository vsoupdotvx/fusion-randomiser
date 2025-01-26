.section .text

HASH_U32 = 0x1758F99D

"Card::Start(&mut self)":
	call replace_card_unlock
"ENDCard::Start(&mut self)":

replace_card_unlock:
	movq   %rbx,      0x10(%rsp)
	
	movslq Card.unlockLevel(%rcx), %rax
	
	movl   $16,  %edx
	cmpl   $"Card::Unlock::EndoFlame", %eax
	cmoveq %rdx, %rax
	
	imull  $0x2493, level_idx(%rip), %edx #this immediate is 65536 / 7 (rounded up), used for modulo
	shrl   $16,             %edx
	imull  $-7,    %edx,    %edx
	addl   level_idx(%rip), %edx
	subl   $5,              %edx
	cmpl   $-3,             %edx
	sbbq   %rdx,            %rdx
	orq    $"Card::Unlock::Unlocked", %rdx
	cmpl   $"Card::Unlock::CattailGirl", %eax
	cmoveq %rdx,            %rax
	
	movl   $"Card::Unlock::Advantrue45"+1, %edx
	cmpl   $"Card::Unlock::Imitater", %eax
	cmoveq %rdx, %rax
	
	cmpq $"Card::Unlock::Unlocked", %rax
	leaq plant_lut(%rip), %rdx
	jle  replace_card_unlock.locA
		movzbl -1(%rdx,%rax), %eax
	replace_card_unlock.locA:
	movl %eax, Card.unlockLevel(%rcx)
	ret

"Advanture_Btn::OnMouseUp(&mut self)+0x4e":
	call adventure_level_enter
	nop
"ENDAdvanture_Btn::OnMouseUp(&mut self)+0x4e":

"PrizeMgr::Click(&mut self)+0x433":
	call set_level_trophy
"ENDPrizeMgr::Click(&mut self)+0x433":

set_level_trophy:
	cmpl $"LevelType::Advanture", GameAPP.theBoardType(%rax)
	jne  set_level_trophy.locA
		movl level_idx(%rip), %edx
	set_level_trophy.locA:
	movb $1, 0x20(%rcx,%rdx)
	ret

adventure_level_enter:
	movl   %edx, level_idx(%rip)
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

init_hash_table_u32_u32: #log2(table_size) in cl, table in rdx, array in r8, init array in r9, array length in r10
	pushq %rdi
	pushq %rsi
	pushq %rbx
	
	negb %cl
	addb $32, %cl
	
	testq %r10, %r10
	je    init_hash_table_u32_u32.exloopA
	xorl  %ebx, %ebx
	init_hash_table_u32_u32.loopA:
		incl %ebx
		
		movl  -8(%r9,%rbx,8),  %esi
		imull $HASH_U32, %esi, %edi
		shrl  %cl,             %edi
		
		movl (%rdx,%rdi,4),  %eax
		movl %esi,          (%r8)
		movl %eax,         8(%r8)
		movl -4(%r9,%rbx,8), %eax
		movl %eax,         4(%r8)
		movl %ebx,  (%rdx,%rdi,4)
		
		addq $12, %r8
		decq %r10
	jne init_hash_table_u32_u32.loopA
	init_hash_table_u32_u32.exloopA:
	
	popq  %rbx
	popq  %rsi
	popq  %rdi
	ret

plant_type_flatten_menu: #for ease of use, this function saves all registers except ecx (input value) and eax (output value)
	pushq %r8
	pushq %r9
	pushq %rdx
	
	cmpb $0, menu_table_initialized(%rip)
	
	jne plant_type_flatten_menu.locA
		pushq %rcx
		pushq %r10
		movb  $6,                %cl
		leaq  menu_table(%rip), %rdx
		leaq  menu_array(%rip),  %r8
		leaq  menu_init_array(%rip), %r9
		movl  $48,             %r10d
		call  init_hash_table_u32_u32
		movb  $1, menu_table_initialized(%rip)
		popq  %r10
		popq  %rcx
	plant_type_flatten_menu.locA:
	
	imull $HASH_U32,  %ecx, %eax
	leaq  menu_table(%rip), %rdx
	leaq  menu_array(%rip),  %r8
	shrl  $26,              %eax
	movl  (%rdx,%rax,4),    %eax
	movq  $-1,              %rdx
	testl %eax,             %eax
	je    plant_type_flatten_menu.exloopA
		plant_type_flatten_menu.loopA:
			leal  (%eax,%eax,2),   %eax
			cmpl  %ecx, -12(%r8,%rax,4)
			movl  -8(%r8,%rax,4),  %edx
			je    plant_type_flatten_menu.exloopA
			movl  -4(%r8,%rax,4), %eax
			testl %eax,           %eax
		jne plant_type_flatten_menu.loopA
		movq $-1,  %rdx
	plant_type_flatten_menu.exloopA:
	movq %rdx, %rax
	
	popq %rdx
	popq %r9
	popq %r8
	ret

.section .data
level_idx:
	.long 0
menu_init_array:
	.long "MixData::PlantType::Peashooter";    .long 0
	.long "MixData::PlantType::SunFlower";     .long 1
	.long "MixData::PlantType::CherryBomb";    .long 2
	.long "MixData::PlantType::WallNut";       .long 3
	.long "MixData::PlantType::PotatoMine";    .long 4
	.long "MixData::PlantType::Chomper";       .long 5
	
	.long "MixData::PlantType::SmallPuff";     .long 6
	.long "MixData::PlantType::FumeShroom";    .long 7
	.long "MixData::PlantType::HypnoShroom";   .long 8
	.long "MixData::PlantType::ScaredyShroom"; .long 9
	.long "MixData::PlantType::IceShroom";     .long 10
	.long "MixData::PlantType::DoomShroom";    .long 11
	
	.long "MixData::PlantType::LilyPad";       .long 12
	.long "MixData::PlantType::Squash";        .long 13
	.long "MixData::PlantType::ThreePeater";   .long 14
	.long "MixData::PlantType::Tanglekelp";    .long 15
	.long "MixData::PlantType::Jalapeno";      .long 16
	.long "MixData::PlantType::Caltrop";       .long 17
	.long "MixData::PlantType::TorchWood";     .long 18
	
	.long "MixData::PlantType::SeaShroom";     .long 19
	.long "MixData::PlantType::Plantern";      .long 20
	.long "MixData::PlantType::Cactus";        .long 21
	.long "MixData::PlantType::Blower";        .long 22
	.long "MixData::PlantType::StarFruit";     .long 23
	.long "MixData::PlantType::Pumpkin";       .long 24
	.long "MixData::PlantType::Magnetshroom";  .long 25
	
	.long "MixData::PlantType::Cabbagepult";   .long 26
	.long "MixData::PlantType::Pot";           .long 27
	.long "MixData::PlantType::Cornpult";      .long 28
	.long "MixData::PlantType::Garlic";        .long 29
	.long "MixData::PlantType::Umbrellaleaf";  .long 30
	.long "MixData::PlantType::Marigold";      .long 31
	.long "MixData::PlantType::Melonpult";     .long 32
	
	.long "MixData::PlantType::PresentZombie"; .long 33
	.long "MixData::PlantType::EndoFlame";     .long 34
	.long "MixData::PlantType::Present";       .long 35
	.long "MixData::PlantType::TallNut";       .long 36
	.long "MixData::PlantType::SpikeRock";     .long 37
	.long "MixData::PlantType::CattailPlant";  .long 38
	.long "MixData::PlantType::GloomShroom";   .long 39
	.long "MixData::PlantType::CobCannon";     .long 40
	
	.long "MixData::PlantType::Imitater";      .long 41
	.long "MixData::PlantType::Squalour";      .long 42
	.long "MixData::PlantType::SwordStar";     .long 43
	.long "MixData::PlantType::BigSunNut";     .long 44
	.long "MixData::PlantType::CattailGirl";   .long 45
	.long "MixData::PlantType::Wheat";         .long 46
	.long "MixData::PlantType::BigWallNut";    .long 47

menu_table:
	.space 0x40 * 4
menu_array:
	.space 48 * 12
level_lut:
	.space 50, 0x2
plant_lut:
	.space 50, 0x0
menu_table_initialized:
	.byte 0

