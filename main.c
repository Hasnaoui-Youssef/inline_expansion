#include <stdio.h>
#include <string.h>

int foo(int,const char*,int);

void bar(const char *,int);

int main(){

    int x = 10;
    const char* str = "Hello";
    int len = strlen(str);
    int y = foo(x, str, len);
    printf("Foo return : %d\n", y);
}

int foo(int x, const char* str, int len) {
    if(x < 10){
        printf("X less than 10\n");
        return -1;
    }

    bar(str, len);
    return printf("%s has %d chars\n", str, len);
}

void bar(const char* str, int len){
    int count = 0;
    char* c = str;
    for(int i = 0; (*c++) && i < len; ++i){
        count++;
    }
    printf("Matching count ? %s\n", (c ? "No" : "Yes"));
}
