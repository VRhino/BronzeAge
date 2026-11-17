using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using BronzeAge.Contract.V1;
using Google.Protobuf;
using Xunit;

// Ficheros dorados: el JSON es la fuente, el .bin la codificación canónica (generada por contratos/ts).
public class GoldenTests
{
    static readonly string Dir = Path.Combine(AppContext.BaseDirectory, "golden");

    public static IEnumerable<object[]> Casos() =>
        Directory.GetFiles(Dir, "*.json").Select(f => new object[] { Path.GetFileNameWithoutExtension(f) });

    static IMessage DesdeJson(string tipo, string json) => tipo switch
    {
        "MensajeDeCliente" => JsonParser.Default.Parse<MensajeDeCliente>(json),
        "MensajeDeServidor" => JsonParser.Default.Parse<MensajeDeServidor>(json),
        "ArtefactoDeMundo" => JsonParser.Default.Parse<ArtefactoDeMundo>(json),
        _ => throw new ArgumentException(tipo),
    };

    static IMessage DesdeBin(string tipo, byte[] bin) => tipo switch
    {
        "MensajeDeCliente" => MensajeDeCliente.Parser.ParseFrom(bin),
        "MensajeDeServidor" => MensajeDeServidor.Parser.ParseFrom(bin),
        "ArtefactoDeMundo" => ArtefactoDeMundo.Parser.ParseFrom(bin),
        _ => throw new ArgumentException(tipo),
    };

    [Theory, MemberData(nameof(Casos))]
    public void JsonYBinarioCoinciden(string nombre)
    {
        var tipo = nombre.Split("__")[0];
        var bin = File.ReadAllBytes(Path.Combine(Dir, nombre + ".bin"));
        var desdeJson = DesdeJson(tipo, File.ReadAllText(Path.Combine(Dir, nombre + ".json")));
        Assert.Equal(bin, desdeJson.ToByteArray());
        Assert.Equal(bin, DesdeBin(tipo, bin).ToByteArray());
    }
}
