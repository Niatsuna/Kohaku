# Simple Delegation for convenience
.PHONE: fmt check test

fmt: 
	@$(MAKE) -C .. fmt-server

check: 
	@$(MAKE) -C .. check-server

test:
	@$(MAKE) -C .. test-server

fmt-server: fmt
check-server: check
test-server: test