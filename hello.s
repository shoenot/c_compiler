	.globl main
main:
	pushq	%rbp
	movq	%rsp,	%rbp
	subq	$112,	%rsp
	movl	$72,	-4(%rbp)
	subq	$8,	%rsp
	movl	-4(%rbp),	%edi
	call	putchar@PLT
	addq	$8,	%rsp
	movl	%eax,	-8(%rbp)
	movl	$101,	-12(%rbp)
	subq	$8,	%rsp
	movl	-12(%rbp),	%edi
	call	putchar@PLT
	addq	$8,	%rsp
	movl	%eax,	-16(%rbp)
	movl	$108,	-20(%rbp)
	subq	$8,	%rsp
	movl	-20(%rbp),	%edi
	call	putchar@PLT
	addq	$8,	%rsp
	movl	%eax,	-24(%rbp)
	movl	$108,	-28(%rbp)
	subq	$8,	%rsp
	movl	-28(%rbp),	%edi
	call	putchar@PLT
	addq	$8,	%rsp
	movl	%eax,	-32(%rbp)
	movl	$111,	-36(%rbp)
	subq	$8,	%rsp
	movl	-36(%rbp),	%edi
	call	putchar@PLT
	addq	$8,	%rsp
	movl	%eax,	-40(%rbp)
	movl	$44,	-44(%rbp)
	subq	$8,	%rsp
	movl	-44(%rbp),	%edi
	call	putchar@PLT
	addq	$8,	%rsp
	movl	%eax,	-48(%rbp)
	movl	$32,	-52(%rbp)
	subq	$8,	%rsp
	movl	-52(%rbp),	%edi
	call	putchar@PLT
	addq	$8,	%rsp
	movl	%eax,	-56(%rbp)
	movl	$87,	-60(%rbp)
	subq	$8,	%rsp
	movl	-60(%rbp),	%edi
	call	putchar@PLT
	addq	$8,	%rsp
	movl	%eax,	-64(%rbp)
	movl	$111,	-68(%rbp)
	subq	$8,	%rsp
	movl	-68(%rbp),	%edi
	call	putchar@PLT
	addq	$8,	%rsp
	movl	%eax,	-72(%rbp)
	movl	$114,	-76(%rbp)
	subq	$8,	%rsp
	movl	-76(%rbp),	%edi
	call	putchar@PLT
	addq	$8,	%rsp
	movl	%eax,	-80(%rbp)
	movl	$108,	-84(%rbp)
	subq	$8,	%rsp
	movl	-84(%rbp),	%edi
	call	putchar@PLT
	addq	$8,	%rsp
	movl	%eax,	-88(%rbp)
	movl	$100,	-92(%rbp)
	subq	$8,	%rsp
	movl	-92(%rbp),	%edi
	call	putchar@PLT
	addq	$8,	%rsp
	movl	%eax,	-96(%rbp)
	movl	$33,	-100(%rbp)
	subq	$8,	%rsp
	movl	-100(%rbp),	%edi
	call	putchar@PLT
	addq	$8,	%rsp
	movl	%eax,	-104(%rbp)
	movl	$10,	-108(%rbp)
	subq	$8,	%rsp
	movl	-108(%rbp),	%edi
	call	putchar@PLT
	addq	$8,	%rsp
	movl	%eax,	-112(%rbp)
	movl	$0,	%eax
	movq	%rbp,	%rsp
	popq	%rbp
	ret

.section .note.GNU-stack,"",@progbits