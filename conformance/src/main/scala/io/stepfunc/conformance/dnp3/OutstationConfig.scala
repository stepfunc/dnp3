package io.stepfunc.conformance.dnp3

case class TcpConfig(address: String, port: Int)

case class LinkConfig(source: Int, destination: Int, timeoutMs: Int, selfAddressSupport: Boolean)

case class OutstationConfig(responseTimeoutMs: Int, selectTimeoutMs: Int, fragmentSize: Int, maxControlsPerRequest: Int)

case class UnsolicitedResponseConfig(allowUnsolicited: Boolean, unsolConfirmTimeoutMs: Int, maxNumRetries: Option[Int])

case class TestDatabaseConfig(disableBinaryInputs: Boolean, disableDoubleBitBinaryInputs: Boolean, disableCounters: Boolean, isGlobalLocalControl: Boolean, isSingleLocalControl: Boolean)

case class CommandHandlerConfig(disableBinaryOutput: Boolean, disableAnalogOutput: Boolean)

case class StackConfig(tcpConfig: TcpConfig, linkConfig: LinkConfig, outstationConfig: OutstationConfig, unsolicitedResponseConfig: UnsolicitedResponseConfig, testDatabaseConfig: TestDatabaseConfig, commandHandlerConfig: CommandHandlerConfig)

object StackConfig {
  val Default = StackConfig(
    TcpConfig("127.0.0.1", 20000),
    LinkConfig(1024, 1, 1000, false),
    OutstationConfig(2000, 2000, 2048, 4),
    UnsolicitedResponseConfig(allowUnsolicited = false, 5000, None),
    TestDatabaseConfig(disableBinaryInputs = false, disableDoubleBitBinaryInputs = false, disableCounters = false, isGlobalLocalControl = false, isSingleLocalControl = false),
    CommandHandlerConfig(disableBinaryOutput = false, disableAnalogOutput = false)
  )
}
