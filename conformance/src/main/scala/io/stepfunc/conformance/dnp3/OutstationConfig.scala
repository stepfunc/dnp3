package io.stepfunc.conformance.dnp3

case class TcpConfig(address: String, port: Int)

case class LinkConfig(source: Int, destination: Int, timeoutMs: Int, selfAddressSupport: Boolean)

// confirmTimeoutMs: the outstation has one confirm timeout, for solicited and unsolicited responses
case class OutstationConfig(confirmTimeoutMs: Int, selectTimeoutMs: Int, fragmentSize: Int, maxControlsPerRequest: Int)

// retryDelayMs: the wait after a failed unsolicited response series before starting another
case class UnsolicitedResponseConfig(allowUnsolicited: Boolean, retryDelayMs: Int, maxNumRetries: Option[Int])

case class TestDatabaseConfig(disableBinaryInputs: Boolean, disableDoubleBitBinaryInputs: Boolean, disableCounters: Boolean, isGlobalLocalControl: Boolean, isSingleLocalControl: Boolean)

case class CommandHandlerConfig(disableBinaryOutput: Boolean, disableAnalogOutput: Boolean)

case class StackConfig(tcpConfig: TcpConfig, linkConfig: LinkConfig, outstationConfig: OutstationConfig, unsolicitedResponseConfig: UnsolicitedResponseConfig, testDatabaseConfig: TestDatabaseConfig, commandHandlerConfig: CommandHandlerConfig)

object StackConfig {
  val Default = StackConfig(
    TcpConfig("127.0.0.1", 20000),
    LinkConfig(1024, 1, 1000, false),
    OutstationConfig(2000, 2000, 2048, 4),
    UnsolicitedResponseConfig(allowUnsolicited = false, 2000, None),
    TestDatabaseConfig(disableBinaryInputs = false, disableDoubleBitBinaryInputs = false, disableCounters = false, isGlobalLocalControl = false, isSingleLocalControl = false),
    CommandHandlerConfig(disableBinaryOutput = false, disableAnalogOutput = false)
  )
}
