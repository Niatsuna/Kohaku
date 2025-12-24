# Simple Delegation for convenience
.PHONE: fmt check test

fmt: 
	@$(MAKE) -C .. fmt-client

check: 
	@$(MAKE) -C .. check-client

test:
	@$(MAKE) -C .. test-client

fmt-client: fmt
check-client: check
test-client: test