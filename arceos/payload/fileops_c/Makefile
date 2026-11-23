TARGET := fileops

CC := riscv64-linux-musl-gcc
STRIP := riscv64-linux-musl-strip

all: $(TARGET)

$(TARGET): $(TARGET).c
	@if command -v $(CC) >/dev/null 2>&1; then \
		$(CC) -static $< -o $@ && $(STRIP) $@; \
	else \
		echo "warning: $(CC) not found, skipping $@"; \
	fi

clean:
	@rm -rf ./$(TARGET)
