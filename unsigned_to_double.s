	.file	"unsigned_to_double.c"
	.text
	.globl	uint_to_double
	.type	uint_to_double, @function
uint_to_double:
.LFB0:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	movl	%edi, -4(%rbp)
	movl	-4(%rbp), %eax
	testq	%rax, %rax
	js	.L2
	pxor	%xmm0, %xmm0
	cvtsi2sdq	%rax, %xmm0
	jmp	.L4
.L2:
	movq	%rax, %rdx
	shrq	%rdx
	andl	$1, %eax
	orq	%rax, %rdx
	pxor	%xmm0, %xmm0
	cvtsi2sdq	%rdx, %xmm0
	addsd	%xmm0, %xmm0
.L4:
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE0:
	.size	uint_to_double, .-uint_to_double
	.globl	ulong_to_double
	.type	ulong_to_double, @function
ulong_to_double:
.LFB1:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	movq	%rdi, -8(%rbp)
	movq	-8(%rbp), %rax
	testq	%rax, %rax
	js	.L6
	pxor	%xmm0, %xmm0
	cvtsi2sdq	%rax, %xmm0
	jmp	.L8
.L6:
	movq	%rax, %rdx
	shrq	%rdx
	andl	$1, %eax
	orq	%rax, %rdx
	pxor	%xmm0, %xmm0
	cvtsi2sdq	%rdx, %xmm0
	addsd	%xmm0, %xmm0
.L8:
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE1:
	.size	ulong_to_double, .-ulong_to_double
	.globl	main
	.type	main, @function
main:
.LFB2:
	.cfi_startproc
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset 6, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register 6
	movl	$1000, %edi
	call	uint_to_double
	movq	%xmm0, %rax
	movq	%rax, %xmm1
	ucomisd	.LC0(%rip), %xmm1
	jp	.L27
	movq	%rax, %xmm2
	ucomisd	.LC0(%rip), %xmm2
	je	.L10
.L27:
	movl	$1, %eax
	jmp	.L12
.L10:
	movl	$-96, %edi
	call	uint_to_double
	movq	%xmm0, %rax
	movq	%rax, %xmm3
	ucomisd	.LC1(%rip), %xmm3
	jp	.L28
	movq	%rax, %xmm4
	ucomisd	.LC1(%rip), %xmm4
	je	.L13
.L28:
	movl	$2, %eax
	jmp	.L12
.L13:
	movabsq	$138512825844, %rax
	movq	%rax, %rdi
	call	ulong_to_double
	movq	%xmm0, %rax
	movq	%rax, %xmm5
	ucomisd	.LC2(%rip), %xmm5
	jp	.L29
	movq	%rax, %xmm6
	ucomisd	.LC2(%rip), %xmm6
	je	.L15
.L29:
	movl	$3, %eax
	jmp	.L12
.L15:
	movabsq	$-8223372036854775800, %rax
	movq	%rax, %rdi
	call	ulong_to_double
	movq	%xmm0, %rax
	movq	%rax, %xmm7
	ucomisd	.LC3(%rip), %xmm7
	jp	.L30
	movq	%rax, %xmm1
	ucomisd	.LC3(%rip), %xmm1
	je	.L17
.L30:
	movl	$4, %eax
	jmp	.L12
.L17:
	movabsq	$-9223372036854774784, %rax
	movq	%rax, %rdi
	call	ulong_to_double
	movq	%xmm0, %rax
	movq	%rax, %xmm2
	ucomisd	.LC4(%rip), %xmm2
	jp	.L31
	movq	%rax, %xmm3
	ucomisd	.LC4(%rip), %xmm3
	je	.L19
.L31:
	movl	$5, %eax
	jmp	.L12
.L19:
	movabsq	$-9223372036854774783, %rax
	movq	%rax, %rdi
	call	ulong_to_double
	movq	%xmm0, %rax
	movq	%rax, %xmm4
	ucomisd	.LC5(%rip), %xmm4
	jp	.L32
	movq	%rax, %xmm5
	ucomisd	.LC5(%rip), %xmm5
	je	.L21
.L32:
	movl	$6, %eax
	jmp	.L12
.L21:
	movabsq	$-9223372036854774785, %rax
	movq	%rax, %rdi
	call	ulong_to_double
	movq	%xmm0, %rax
	movq	%rax, %xmm6
	ucomisd	.LC4(%rip), %xmm6
	jp	.L33
	movq	%rax, %xmm7
	ucomisd	.LC4(%rip), %xmm7
	je	.L23
.L33:
	movl	$7, %eax
	jmp	.L12
.L23:
	movabsq	$-9223372036854774786, %rax
	movq	%rax, %rdi
	call	ulong_to_double
	movq	%xmm0, %rax
	movq	%rax, %xmm1
	ucomisd	.LC4(%rip), %xmm1
	jp	.L34
	movq	%rax, %xmm2
	ucomisd	.LC4(%rip), %xmm2
	je	.L25
.L34:
	movl	$8, %eax
	jmp	.L12
.L25:
	movl	$0, %eax
.L12:
	popq	%rbp
	.cfi_def_cfa 7, 8
	ret
	.cfi_endproc
.LFE2:
	.size	main, .-main
	.section	.rodata
	.align 8
.LC0:
	.long	0
	.long	1083129856
	.align 8
.LC1:
	.long	-201326592
	.long	1106247679
	.align 8
.LC2:
	.long	-17170432
	.long	1111498752
	.align 8
.LC3:
	.long	-696980352
	.long	1138867222
	.align 8
.LC4:
	.long	0
	.long	1138753536
	.align 8
.LC5:
	.long	1
	.long	1138753536
	.ident	"GCC: (GNU) 15.2.1 20260209"
	.section	.note.GNU-stack,"",@progbits
