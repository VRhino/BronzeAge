using BronzeAge.Contract.V1;
using Google.Protobuf;
using Xunit;

public class SmokeTests
{
    [Fact]
    public void PingIdaYVuelta()
    {
        var m = new MensajeDeCliente { Ping = new Ping { ClienteEnvioMs = 42 } };
        Assert.Equal(m, MensajeDeCliente.Parser.ParseFrom(m.ToByteArray()));
    }
}
