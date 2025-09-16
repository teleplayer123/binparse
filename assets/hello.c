#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(int argc, char *argv[]) {
    size_t i, n;
    unsigned char *msg, *key;
    if(argc != 3) {
        printf("Usage: %s <message> <key>\n", argv[0]);
        return 1;
    }
    msg = (unsigned char *)argv[1];
    key = (unsigned char *)argv[2];
    n = strlen(msg);
    printf("Original message: %s\n", msg);
    for(i = 0; i <= n-1; i++) {
        msg[i] ^= key[i % strlen(key)];
    }
    printf("New message: %s\n", msg);
    return 0;
}