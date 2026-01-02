#include "main.h"

extern UART_HandleTypeDef huart2;

void send_char(char c){
    HAL_UART_Transmit(&huart2, (uint8_t*) &c, sizeof(c), HAL_MAX_DELAY);

}
