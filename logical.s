	.file	"logical.c"
	.text
	.globl	zero
	.bss
	.align 8
	.type	zero, @object
	.size	zero, 8
zero:
	.zero	8
	.globl	non_zero
	.data
	.align 8
	.type	non_zero, @object
	.size	non_zero, 8
non_zero:
	.long	210911779
	.long	1002937505
	.globl	one
	.align 8
	.type	one, @object
	.size	one, 8
one:
	.long	0
	.long	1072693248
	.globl	rounded_to_zero
	.bss
	.align 8
	.type	rounded_to_zero, @object
	.size	rounded_to_zero, 8
rounded_to_zero:
	.zero	8
	.text
	.globl	main
	.type	main, @function
main:
.LFB0:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	movsd	zero(%rip), %xmm0
	pxor	%xmm1, %xmm1
	ucomisd	%xmm1, %xmm0
	jp	.L31
	pxor	%xmm1, %xmm1
	ucomisd	%xmm1, %xmm0
	je	.L2
.L31:
	movl	$1, %eax
	jmp	.L4
.L2:
	movsd	rounded_to_zero(%rip), %xmm0
	pxor	%xmm1, %xmm1
	ucomisd	%xmm1, %xmm0
	jp	.L32
	pxor	%xmm1, %xmm1
	ucomisd	%xmm1, %xmm0
	je	.L5
.L32:
	movl	$2, %eax
	jmp	.L4
.L5:
	movsd	non_zero(%rip), %xmm0
	pxor	%xmm1, %xmm1
	ucomisd	%xmm1, %xmm0
	jp	.L7
	pxor	%xmm1, %xmm1
	ucomisd	%xmm1, %xmm0
	jne	.L7
	movl	$3, %eax
	jmp	.L4
.L7:
	movsd	non_zero(%rip), %xmm0
	pxor	%xmm1, %xmm1
	ucomisd	%xmm1, %xmm0
	jp	.L9
	pxor	%xmm1, %xmm1
	ucomisd	%xmm1, %xmm0
	jne	.L9
	movl	$5, %eax
	jmp	.L4
.L9:
	movsd	zero(%rip), %xmm0
	pxor	%xmm1, %xmm1
	ucomisd	%xmm1, %xmm0
	jp	.L35
	pxor	%xmm1, %xmm1
	ucomisd	%xmm1, %xmm0
	je	.L11
.L35:
	movl	$6, %eax
	jmp	.L4
.L11:
	movsd	rounded_to_zero(%rip), %xmm0
	pxor	%xmm1, %xmm1
	ucomisd	%xmm1, %xmm0
	jp	.L36
	pxor	%xmm1, %xmm1
	ucomisd	%xmm1, %xmm0
	je	.L13
.L36:
	movl	$7, %eax
	jmp	.L4
.L13:
	movsd	non_zero(%rip), %xmm0
	pxor	%xmm1, %xmm1
	ucomisd	%xmm1, %xmm0
	jp	.L15
	pxor	%xmm1, %xmm1
	ucomisd	%xmm1, %xmm0
	jne	.L15
	movl	$8, %eax
	jmp	.L4
.L15:
	movsd	zero(%rip), %xmm0
	pxor	%xmm1, %xmm1
	ucomisd	%xmm1, %xmm0
	jp	.L38
	pxor	%xmm1, %xmm1
	ucomisd	%xmm1, %xmm0
	je	.L17
.L38:
	movl	$9, %eax
	jmp	.L4
.L17:
	movsd	rounded_to_zero(%rip), %xmm0
	pxor	%xmm1, %xmm1
	ucomisd	%xmm1, %xmm0
	jp	.L39
	pxor	%xmm1, %xmm1
	ucomisd	%xmm1, %xmm0
	je	.L19
.L39:
	movl	$10, %eax
	jmp	.L4
.L19:
	movsd	zero(%rip), %xmm0
	pxor	%xmm1, %xmm1
	ucomisd	%xmm1, %xmm0
	jp	.L40
	pxor	%xmm1, %xmm1
	ucomisd	%xmm1, %xmm0
	je	.L21
.L40:
	movl	$11, %eax
	jmp	.L4
.L21:
	movsd	non_zero(%rip), %xmm0
	pxor	%xmm1, %xmm1
	ucomisd	%xmm1, %xmm0
	jp	.L23
	pxor	%xmm1, %xmm1
	ucomisd	%xmm1, %xmm0
	jne	.L23
	movl	$12, %eax
	jmp	.L4
.L23:
	movsd	zero(%rip), %xmm0
	pxor	%xmm1, %xmm1
	ucomisd	%xmm1, %xmm0
	jp	.L25
	pxor	%xmm1, %xmm1
	ucomisd	%xmm1, %xmm0
	jne	.L25
	movsd	rounded_to_zero(%rip), %xmm0
	pxor	%xmm1, %xmm1
	ucomisd	%xmm1, %xmm0
	jp	.L25
	pxor	%xmm1, %xmm1
	ucomisd	%xmm1, %xmm0
	je	.L27
.L25:
	movl	$14, %eax
	jmp	.L4
.L27:
	movsd	non_zero(%rip), %xmm0
	pxor	%xmm1, %xmm1
	ucomisd	%xmm1, %xmm0
	jp	.L29
	pxor	%xmm1, %xmm1
	ucomisd	%xmm1, %xmm0
	jne	.L29
	movl	$16, %eax
	jmp	.L4
.L29:
	movl	$0, %eax
.L4:
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE0:
	.size	main, .-main
	.ident	"GCC: (GNU) 15.2.1 20260209"
	.section	.note.GNU-stack,"",@progbits
