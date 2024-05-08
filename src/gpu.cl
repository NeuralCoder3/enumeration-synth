// .name("apply")
// .queue(ctx.queue().clone())
// .global_work_size(frontier.len())
// .arg(&state_buffer)
// .arg(&command_buffer)
// .arg(&output_buffer)
// .arg(&state_size)
// .arg(&permutation_size)
__kernel void apply(__global uchar* data, __global uchar* command, __global uchar* output) {//, int state_size, int permutation_size) {
    // int state_size = 36;
    // int permutation_size = 6;
    #define CMP 0
    #define MOV 1
    #define CMOVG 2
    #define CMOVL 3

    #define perm_count 6
    #define permutation_size 6
    #define state_size perm_count * permutation_size
    int gid = get_global_id(0);

    int instruction = command[0];
    int a = command[1];
    int b = command[2];

    int state_index = gid * state_size;
    for (int perm_offset = 0; perm_offset < state_size; perm_offset += permutation_size) {
        int perm_index = state_index + perm_offset;

        int to = perm_index + a;
        int from = perm_index + b;

        int lt_flag = perm_index + permutation_size - 2;
        int gt_flag = perm_index + permutation_size - 1;

        switch (instruction) {
            case CMP:
                data[lt_flag] = data[to] < data[from];
                data[gt_flag] = data[to] > data[from];
                break;
            case MOV:
                data[to] = data[from];
                break;
            case CMOVG:
                if(data[gt_flag])
                    data[to] = data[from];
                break;
            case CMOVL:
                if(data[lt_flag])
                    data[to] = data[from];
                break;
        }
    }

}