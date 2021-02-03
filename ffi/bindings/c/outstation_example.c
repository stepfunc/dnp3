#include "dnp3rs.h"

#include <stdio.h>
#include <string.h>
#include <time.h>

// Logger callback
void on_log_message(log_level_t level, const char* msg, void* arg)
{    
    printf("%s", msg);
}

// Application callbacks
uint16_t get_processing_delay_ms(void* context)
{
    return 0;
}

restart_delay_t cold_restart(void* context)
{
    return restart_delay_seconds(60);
}

restart_delay_t warm_restart(void* context)
{
    return restart_delay_not_supported();
}

// Outstation information callbacks
void process_request_from_idle(request_header_t header, void* context)
{

}

void broadcast_received(function_code_t function_code, broadcast_action_t action, void* context)
{

}

void enter_solicited_confirm_wait(uint8_t ecsn, void* context)
{

}

void solicited_confirm_timeout(uint8_t ecsn, void* context)
{

}

void solicited_confirm_received(uint8_t ecsn, void* context)
{

}

void solicited_confirm_wait_new_request(request_header_t header, void* context)
{

}

void wrong_solicited_confirm_seq(uint8_t ecsn, uint8_t seq, void* context)
{

}

void unexpected_confirm(bool unsolicited, uint8_t seq, void* context)
{

}

void enter_unsolicited_confirm_wait(uint8_t ecsn, void* context)
{

}

void unsolicited_confirm_timeout(uint8_t ecsn, bool retry, void* context)
{

}

void unsolicited_confirmed(uint8_t ecsn, void* context)
{

}

void clear_restart_iin(void* context)
{

}

// Control handler callbacks
void begin_fragment(void* context)
{

}

void end_fragment(void* context)
{

}

command_status_t select_g12v1(g12v1_t control, uint16_t index, database_t* database, void* context)
{
    return CommandStatus_NotSupported;
}

command_status_t operate_g12v1(g12v1_t control, uint16_t index, operate_type_t op_type, database_t* database, void* context)
{
    return CommandStatus_NotSupported;
}

command_status_t select_g41v1(int32_t control, uint16_t index, database_t* database, void* context)
{
    return CommandStatus_NotSupported;
}

command_status_t operate_g41v1(int32_t control, uint16_t index, operate_type_t op_type, database_t* database, void* context)
{
    return CommandStatus_NotSupported;
}

command_status_t select_g41v2(int16_t control, uint16_t index, database_t* database, void* context)
{
    return CommandStatus_NotSupported;
}

command_status_t operate_g41v2(int16_t control, uint16_t index, operate_type_t op_type, database_t* database, void* context)
{
    return CommandStatus_NotSupported;
}

command_status_t select_g41v3(float control, uint16_t index, database_t* database, void* context)
{
    return CommandStatus_NotSupported;
}

command_status_t operate_g41v3(float control, uint16_t index, operate_type_t op_type, database_t* database, void* context)
{
    return CommandStatus_NotSupported;
}

command_status_t select_g41v4(double control, uint16_t index, database_t* database, void* context)
{
    return CommandStatus_NotSupported;
}

command_status_t operate_g41v4(double control, uint16_t index, operate_type_t op_type, database_t* database, void* context)
{
    return CommandStatus_NotSupported;
}

// Transactions
void outstation_transaction_startup(database_t* db, void* context)
{
    for(uint16_t i = 0; i < 10; ++i)
    {
        // Create the points
        binary_config_t binary_config =
        {
            .static_variation = StaticBinaryVariation_Group1Var2,
            .event_variation = EventBinaryVariation_Group2Var2,
        };
        database_add_binary(db, i, EventClass_Class1, binary_config);

        double_bit_binary_config_t double_bit_binary_config =
        {
            .static_variation = StaticDoubleBitBinaryVariation_Group3Var2,
            .event_variation = EventDoubleBitBinaryVariation_Group4Var2,
        };
        database_add_double_bit_binary(db, i, EventClass_Class1, double_bit_binary_config);

        binary_output_status_config_t binary_output_status_config =
        {
            .static_variation = StaticBinaryOutputStatusVariation_Group10Var2,
            .event_variation = EventBinaryOutputStatusVariation_Group11Var2,
        };
        database_add_binary_output_status(db, i, EventClass_Class1, binary_output_status_config);

        counter_config_t counter_config =
        {
            .static_variation = StaticCounterVariation_Group20Var1,
            .event_variation = EventCounterVariation_Group22Var1,
            .deadband = 0,
        };
        database_add_counter(db, i, EventClass_Class1, counter_config);

        frozen_counter_config_t frozen_counter_config =
        {
            .static_variation = StaticFrozenCounterVariation_Group21Var5,
            .event_variation = EventFrozenCounterVariation_Group23Var5,
            .deadband = 0,
        };
        database_add_frozen_counter(db, i, EventClass_Class1, frozen_counter_config);

        analog_config_t analog_config =
        {
            .static_variation = StaticAnalogVariation_Group30Var6,
            .event_variation = EventAnalogVariation_Group32Var8,
            .deadband = 0.0,
        };
        database_add_analog(db, i, EventClass_Class1, analog_config);

        analog_output_status_config_t analog_output_status_config =
        {
            .static_variation = StaticAnalogOutputStatusVariation_Group40Var4,
            .event_variation = EventAnalogOutputStatusVariation_Group42Var8,
            .deadband = 0.0,
        };
        database_add_analog_output_status(db, i, EventClass_Class1, analog_output_status_config);

        database_add_octet_string(db, i, EventClass_Class1);

        // Set initial values
        flags_t flags = flags_init(0x00);
        flags = flags_set(&flags, Flag_Restart, true);

        binary_t binary_value = binary_init(
            i,
            false,
            flags,
            timestamp_invalid()
        );
        database_update_binary(db, binary_value, update_options_init());

        double_bit_binary_t double_bit_binary_value = double_bit_binary_init(
            i,
            DoubleBit_DeterminedOff,
            flags,
            timestamp_invalid()
        );
        database_update_double_bit_binary(db, double_bit_binary_value, update_options_init());

        binary_output_status_t binary_output_status_value = binary_output_status_init(
            i,
            false,
            flags,
            timestamp_invalid()
        );
        database_update_binary_output_status(db, binary_output_status_value, update_options_init());

        counter_t counter_value = counter_init(
            i,
            0,
            flags,
            timestamp_invalid()
        );
        database_update_counter(db, counter_value, update_options_init());

        frozen_counter_t frozen_counter_value = frozen_counter_init(
            i,
            0,
            flags,
            timestamp_invalid()
        );
        database_update_frozen_counter(db, frozen_counter_value, update_options_init());

        analog_t analog_value = analog_init(
            i,
            0.0,
            flags,
            timestamp_invalid()
        );
        database_update_analog(db, analog_value, update_options_init());

        analog_output_status_t analog_output_status_value = analog_output_status_init(
            i,
            0.0,
            flags,
            timestamp_invalid()
        );
        database_update_analog_output_status(db, analog_output_status_value, update_options_init());
    }
}

typedef struct database_points_t
{
    bool binaryValue;
    double_bit_t doubleBitBinaryValue;
    bool binaryOutputStatusValue;
    uint32_t counterValue;
    uint32_t frozenCounterValue;
    double analogValue;
    double analogOutputStatusValue;
} database_points_t;

flags_t online_flag()
{
    flags_t result = flags_init(0x00);
    return flags_set(&result, Flag_Online, true);
}

void binary_transaction(database_t* db, void* context)
{
    ((database_points_t*)context)->binaryValue = !((database_points_t*)context)->binaryValue;

    binary_t value = binary_init(
        7,
        ((database_points_t*)context)->binaryValue,
        online_flag(),
        timestamp_synchronized(0)
    );
    database_update_binary(db, value, update_options_init());
}

void double_bit_binary_transaction(database_t* db, void* context)
{
    ((database_points_t*)context)->doubleBitBinaryValue = ((database_points_t*)context)->doubleBitBinaryValue == DoubleBit_DeterminedOff ? DoubleBit_DeterminedOn : DoubleBit_DeterminedOff;

    double_bit_binary_t value = double_bit_binary_init(
        7,
        ((database_points_t*)context)->doubleBitBinaryValue,
        online_flag(),
        timestamp_synchronized(0)
    );
    database_update_double_bit_binary(db, value, update_options_init());
}

void binary_output_status_transaction(database_t* db, void* context)
{
    ((database_points_t*)context)->binaryOutputStatusValue = !((database_points_t*)context)->binaryOutputStatusValue;

    binary_output_status_t value = binary_output_status_init(
        7,
        ((database_points_t*)context)->binaryOutputStatusValue,
        online_flag(),
        timestamp_synchronized(0)
    );
    database_update_binary_output_status(db, value, update_options_init());
}

void counter_transaction(database_t* db, void* context)
{
    counter_t value = counter_init(
        7,
        ++((database_points_t*)context)->counterValue,
        online_flag(),
        timestamp_synchronized(0)
    );
    database_update_counter(db, value, update_options_init());
}

void frozen_counter_transaction(database_t* db, void* context)
{
    frozen_counter_t value = frozen_counter_init(
        7,
        ++((database_points_t*)context)->frozenCounterValue,
        online_flag(),
        timestamp_synchronized(0)
    );
    database_update_frozen_counter(db, value, update_options_init());
}

void analog_transaction(database_t* db, void* context)
{
    analog_t value = analog_init(
        7,
        ++((database_points_t*)context)->analogValue,
        online_flag(),
        timestamp_synchronized(0)
    );
    database_update_analog(db, value, update_options_init());
}

void analog_output_status_transaction(database_t* db, void* context)
{
    analog_output_status_t value = analog_output_status_init(
        7,
        ++((database_points_t*)context)->analogOutputStatusValue,
        online_flag(),
        timestamp_synchronized(0)
    );
    database_update_analog_output_status(db, value, update_options_init());
}

void octet_string_transaction(database_t* db, void* context)
{
    octet_string_value_t* octet_string = octet_string_new();
    octet_string_add(octet_string, 0x48); // H
    octet_string_add(octet_string, 0x65); // e
    octet_string_add(octet_string, 0x6C); // l
    octet_string_add(octet_string, 0x6C); // l
    octet_string_add(octet_string, 0x6F); // o
    octet_string_add(octet_string, 0x00); // \0

    database_update_octet_string(db, 7, octet_string, update_options_init());

    octet_string_destroy(octet_string);
}

int main()
{
    // Setup logging
    logger_t logger =
    {
        .on_message = &on_log_message,
        .ctx = NULL,
    };
    logging_configuration_t log_config = {
        .level = LogLevel_Info,
        .print_level = true,
        .print_module_info = false,
        .time_format = TimeFormat_System,
        .output_format = LogOutputFormat_Json,
    };
    configure_logging(log_config, logger);

    // Create runtime
    runtime_config_t runtime_config =
    {
        .num_core_threads = 4,
    };
    runtime_t* runtime = runtime_new(runtime_config);

    tcp_server_t* server = tcpserver_new(runtime, "127.0.0.1:20000");

    // ANCHOR: outstation_config
    // create an outstation configuration with default values
    outstation_config_t config = outstation_config_init(
        // outstation address
        1024,
        // master address
        1
    );
    // override the default decode log level
    config.log_level = DecodeLogLevel_ObjectValues;
    // ANCHOR_END: outstation_config

    outstation_application_t application =
    {
        .get_processing_delay_ms = &get_processing_delay_ms,
        .cold_restart = &cold_restart,
        .warm_restart = &warm_restart,
        .on_destroy = NULL,
        .ctx = NULL,
    };

    outstation_information_t information =
    {
        .process_request_from_idle = &process_request_from_idle,
        .broadcast_received = &broadcast_received,
        .enter_solicited_confirm_wait = &enter_solicited_confirm_wait,
        .solicited_confirm_timeout = &solicited_confirm_timeout,
        .solicited_confirm_received = &solicited_confirm_received,
        .solicited_confirm_wait_new_request = &solicited_confirm_wait_new_request,
        .wrong_solicited_confirm_seq = &wrong_solicited_confirm_seq,
        .unexpected_confirm = &unexpected_confirm,
        .enter_unsolicited_confirm_wait = &enter_unsolicited_confirm_wait,
        .unsolicited_confirm_timeout = &unsolicited_confirm_timeout,
        .unsolicited_confirmed = &unsolicited_confirmed,
        .clear_restart_iin = &clear_restart_iin,
        .on_destroy = NULL,
        .ctx = NULL,
    };

    control_handler_t control_handler =
    {
        .begin_fragment = &begin_fragment,
        .end_fragment = &end_fragment,
        .select_g12v1 = &select_g12v1,
        .operate_g12v1 = &operate_g12v1,
        .select_g41v1 = &select_g41v1,
        .operate_g41v1 = &operate_g41v1,
        .select_g41v2 = &select_g41v2,
        .operate_g41v2 = &operate_g41v2,
        .select_g41v3 = &select_g41v3,
        .operate_g41v3 = &operate_g41v3,
        .select_g41v4 = &select_g41v4,
        .operate_g41v4 = &operate_g41v4,
        .on_destroy = NULL,
        .ctx = NULL,
    };
    address_filter_t* address_filter = address_filter_any();
    outstation_t* outstation = tcpserver_add_outstation(server, config, event_buffer_config_all_types(10), application, information, control_handler, address_filter);
    address_filter_destroy(address_filter);

    // Setup initial points
    outstation_transaction_t startup_transaction =
    {
        .execute = &outstation_transaction_startup,
        .ctx = NULL,
    };
    outstation_transaction(outstation, startup_transaction);

    // Start the outstation
    tcpserver_bind(server);

    database_points_t database_points =
    {
        .binaryValue = false,
        .doubleBitBinaryValue = DoubleBit_DeterminedOff,
        .binaryOutputStatusValue = false,
        .counterValue = 0,
        .frozenCounterValue = 0,
        .analogValue = 0.0,
        .analogOutputStatusValue = 0.0,
    };

    char cbuf[6];
    while(true)
    {
        fgets(cbuf, 6, stdin);

        if(strcmp(cbuf, "x\n") == 0)
        {
            goto cleanup;
        }
        else if(strcmp(cbuf, "bi\n") == 0)
        {
            outstation_transaction_t transaction =
            {
                .execute = &binary_transaction,
                .ctx = &database_points,
            };
            outstation_transaction(outstation, transaction);
        }
        else if(strcmp(cbuf, "dbbi\n") == 0)
        {
            outstation_transaction_t transaction =
            {
                .execute = &double_bit_binary_transaction,
                .ctx = &database_points,
            };
            outstation_transaction(outstation, transaction);
        }
        else if(strcmp(cbuf, "bos\n") == 0)
        {
            outstation_transaction_t transaction =
            {
                .execute = &binary_output_status_transaction,
                .ctx = &database_points,
            };
            outstation_transaction(outstation, transaction);
        }
        else if(strcmp(cbuf, "co\n") == 0)
        {
            outstation_transaction_t transaction =
            {
                .execute = &counter_transaction,
                .ctx = &database_points,
            };
            outstation_transaction(outstation, transaction);
        }
        else if(strcmp(cbuf, "fco\n") == 0)
        {
            outstation_transaction_t transaction =
            {
                .execute = &frozen_counter_transaction,
                .ctx = &database_points,
            };
            outstation_transaction(outstation, transaction);
        }
        else if(strcmp(cbuf, "ai\n") == 0)
        {
            outstation_transaction_t transaction =
            {
                .execute = &analog_transaction,
                .ctx = &database_points,
            };
            outstation_transaction(outstation, transaction);
        }
        else if(strcmp(cbuf, "aos\n") == 0)
        {
            outstation_transaction_t transaction =
            {
                .execute = &analog_output_status_transaction,
                .ctx = &database_points,
            };
            outstation_transaction(outstation, transaction);
        }
        else if(strcmp(cbuf, "os\n") == 0)
        {
            outstation_transaction_t transaction =
            {
                .execute = &octet_string_transaction,
                .ctx = &database_points,
            };
            outstation_transaction(outstation, transaction);
        }
        else
        {
            printf("Unknown command\n");
        }
    }

    // Cleanup
cleanup:
    outstation_destroy(outstation);
    tcpserver_destroy(server);
    runtime_destroy(runtime);

    return 0;
}
