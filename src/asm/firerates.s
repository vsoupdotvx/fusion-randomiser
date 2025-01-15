.section .text

"Plant::PlantShootUpdate(&mut self)+0x70":
    call plant_get_firerate
"ENDPlant::PlantShootUpdate(&mut self)+0x70":

plant_get_firerate:
    movl Plant.thePlantType(%rbx), %edx
    cmpl $1160, %edx #MAX_PLANT
    ja   plant_get_firerate.locA
        call     plant_type_flatten
        
        leaq     plant_firerate_table(%rip), %rdx
        movzbq   (%rdx,%rax), %rdx
        shlb     $1,           %dl
        cvtsi2ss %edx,       %xmm6
        mulss    const1over254(%rip), %xmm6
        addss    const1.0(%rip), %xmm6
        jc       plant_get_firerate.locB
            mulss const0.5(%rip), %xmm6
    jmp plant_get_firerate.locB
    plant_get_firerate.locA:
        movss const1.0(%rip), %xmm6
    plant_get_firerate.locB:
    mulss Plant.thePlantAttackInterval(%rbx), %xmm6
    ret

.section .data
const1.0:
    .float 1.0
const0.5:
    .float 0.5
const1over254:
    .float 0.00393700787402
plant_firerate_table:
    .space 33 + 12 + 25 + 161, 0x00
