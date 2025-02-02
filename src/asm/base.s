.section .text

HASH_U32 = 0x1758F99D

"Card::Start(&mut self)+0x1AC":
	call replace_card_unlock
"ENDCard::Start(&mut self)+0x1AC":

"Advanture_Btn::OnMouseUp(&mut self)+0x4E":
	call adventure_level_enter_1
	nop
"ENDAdvanture_Btn::OnMouseUp(&mut self)+0x4E":

"MainMenu_Btn::OnMouseUp(&mut self)+0x13B":
	call adventure_level_enter_2
"ENDMainMenu_Btn::OnMouseUp(&mut self)+0x13B":

"PrizeMgr::Click(&mut self)+0x47E":
	call set_level_trophy
"ENDPrizeMgr::Click(&mut self)+0x47E":

"UIMgr::EnterGame(levelType: i32, levelNumber: i32)+0x332":
	call rerandomise
	nop
"ENDUIMgr::EnterGame(levelType: i32, levelNumber: i32)+0x332":

replace_card_unlock:
	movl  %ecx, %edx
	call  plant_type_flatten_menu
	testq %rax, %rax
	jns   replace_card_unlock.locA
	movl  %edx, %ecx
	jmp   "Lawnf::CheckIfPlantUnlock(thePlantType: PlantType) -> bool"
	replace_card_unlock.locA:
	cmpl  $"PlantType::CattailGirl", %edx
	je    replace_card_unlock.locB
	leaq  plant_lut(%rip), %rcx
	movl  level_idx(%rip), %edx
	cmpb  %dl,      (%rcx,%rax)
	setna %al
	ret
	replace_card_unlock.locB:
	imull $0x2493, level_idx(%rip), %edx #this immediate is 65536 / 7 (rounded up), used for modulo
	shrl  $16,             %edx
	imull $-7,    %edx,    %edx
	addl  level_idx(%rip), %edx
	subl  $5,              %edx
	cmpl  $-3,             %edx
	setnc %al
	ret

set_level_trophy:
	cmpl $"LevelType::Advanture", GameAPP.theBoardType(%rax)
	jne  set_level_trophy.locA
		movl level_idx(%rip), %edx
		incl level_idx(%rip)
	set_level_trophy.locA:
	movb $1, 0x20(%rcx,%rdx)
	ret

adventure_level_enter_1:
	movl   %edx, level_idx(%rip)
	leaq   level_lut(%rip), %rcx
	movsbq (%rcx,%rdx),     %rdx
	xorq   %r8,              %r8
	movl   Advanture_Btn.levelType(%rbx), %ecx
	ret

adventure_level_enter_2:
	leaq   level_lut(%rip), %rdx
	movl   level_idx(%rip), %eax
	movzbl (%rdx,%rax),     %edx
	jmp    "UIMgr::EnterGame(levelType: i32, levelNumber: i32)"

MAX_PLANT = 1178
plant_type_flatten: #This likely needs to be checked every single update
	xorl %eax,  %eax
	
	cmpl $1000, %ecx
	jc   plant_type_flatten.locA
		subl $69, %ecx
	plant_type_flatten.locA:
	
	testl %ecx, %ecx
	sets  %al
	
	cmpl  $900, %ecx
	jc    plant_type_flatten.locB
		subl $99, %ecx
	plant_type_flatten.locB:
	
	negq %rax
	
	cmpl  $800, %ecx
	jc    plant_type_flatten.locC
		subl $541, %ecx
	plant_type_flatten.locC:
	
	cmpl $242, %ecx
	jc   plant_type_flatten.locD
		subl $207, %ecx
	plant_type_flatten.locD:
	
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

wait_on_rust:
	movb $1, stopped(%rip)
	wait_on_rust.locA:
		call "System.Threading::Thread::Yield() -> bool"
		cmpb $0, stopped(%rip)
	jne wait_on_rust.locA
	ret

rerandomise:
	movl %ebp, GameAPP.theBoardType(%rcx)
	movq %rcx, game_app_ptr(%rip)
	call wait_on_rust
	ret

.section .data
game_app_ptr:
	.quad 0
level_idx:
	.long 1
menu_init_array:
	.long "PlantType::Peashooter";    .long 0
	.long "PlantType::SunFlower";     .long 1
	.long "PlantType::CherryBomb";    .long 2
	.long "PlantType::WallNut";       .long 3
	.long "PlantType::PotatoMine";    .long 4
	.long "PlantType::Chomper";       .long 5
	
	.long "PlantType::SmallPuff";     .long 6
	.long "PlantType::FumeShroom";    .long 7
	.long "PlantType::HypnoShroom";   .long 8
	.long "PlantType::ScaredyShroom"; .long 9
	.long "PlantType::IceShroom";     .long 10
	.long "PlantType::DoomShroom";    .long 11
	
	.long "PlantType::LilyPad";       .long 12
	.long "PlantType::Squash";        .long 13
	.long "PlantType::ThreePeater";   .long 14
	.long "PlantType::Tanglekelp";    .long 15
	.long "PlantType::Jalapeno";      .long 16
	.long "PlantType::Caltrop";       .long 17
	.long "PlantType::TorchWood";     .long 18
	
	.long "PlantType::SeaShroom";     .long 19
	.long "PlantType::Plantern";      .long 20
	.long "PlantType::Cactus";        .long 21
	.long "PlantType::Blower";        .long 22
	.long "PlantType::StarFruit";     .long 23
	.long "PlantType::Pumpkin";       .long 24
	.long "PlantType::Magnetshroom";  .long 25
	
	.long "PlantType::Cabbagepult";   .long 26
	.long "PlantType::Pot";           .long 27
	.long "PlantType::Cornpult";      .long 28
	.long "PlantType::Garlic";        .long 29
	.long "PlantType::Umbrellaleaf";  .long 30
	.long "PlantType::Marigold";      .long 31
	.long "PlantType::Melonpult";     .long 32
	
	.long "PlantType::PresentZombie"; .long 33
	.long "PlantType::EndoFlame";     .long 34
	.long "PlantType::Present";       .long 35
	.long "PlantType::TallNut";       .long 36
	.long "PlantType::SpikeRock";     .long 37
	.long "PlantType::CattailPlant";  .long 38
	.long "PlantType::GloomShroom";   .long 39
	.long "PlantType::CobCannon";     .long 40
	
	.long "PlantType::Imitater";      .long 41
	.long "PlantType::Squalour";      .long 42
	.long "PlantType::SwordStar";     .long 43
	.long "PlantType::BigSunNut";     .long 44
	.long "PlantType::CattailGirl";   .long 45
	.long "PlantType::Wheat";         .long 46
	.long "PlantType::BigWallNut";    .long 47

menu_table:
	.space 0x40 * 4
menu_array:
	.space 48 * 12
level_lut:
	.space 50, 22
plant_lut:
	.space 50, 0x0
menu_table_initialized:
	.byte 0
stopped:
	.byte 0

