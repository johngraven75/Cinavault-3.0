export type GoogleCastMedia = {
  host: string;
  url: string;
  title?: string;
  contentType?: string;
  posterUrl?: string;
};

export async function castToGoogleDevice(
  media: GoogleCastMedia,
): Promise<string> {
  const { Client, DefaultMediaReceiver } = await import("castv2-client");

  return new Promise((resolve, reject) => {
    const client = new Client();

    client.connect(media.host, () => {
      client.launch(DefaultMediaReceiver, (err: Error | null, player: any) => {
        if (err) {
          client.close();
          reject(err);
          return;
        }

        player.load(
          {
            contentId: media.url,
            contentType: media.contentType || "video/mp4",
            streamType: "BUFFERED",
            metadata: {
              type: 0,
              metadataType: 0,
              title: media.title || "CinaVault Premium",
              images: media.posterUrl ? [{ url: media.posterUrl }] : [],
            },
          },
          { autoplay: true },
          (loadErr: Error | null) => {
            client.close();
            if (loadErr) reject(loadErr);
            else resolve("Casting started");
          },
        );
      });
    });

    client.on("error", (err: Error) => {
      client.close();
      reject(err);
    });
  });
}
