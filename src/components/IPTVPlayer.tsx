import React, { useState, useRef, useEffect } from "react";

const IPTVPlayer: React.FC<{ streamUrl: string }> = ({ streamUrl }) => {
  const videoRef = useRef<HTMLVideoElement>(null);
  const [isPlaying, setIsPlaying] = useState(false);
  const [volume, setVolume] = useState(0.5);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [error, setError] = useState<string | null>(null);

  const togglePlay = () => {
    if (videoRef.current) {
      if (isPlaying) {
        videoRef.current.pause();
      } else {
        videoRef.current.play();
      }
    }
  };

  const handleVolumeChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const vol = parseFloat(e.target.value);
    setVolume(vol);
  };

  const toggleFullscreen = () => {
    if (!videoRef.current) return;

    if (!isFullscreen) {
      if (videoRef.current.requestFullscreen) {
        videoRef.current.requestFullscreen();
      } else if ((videoRef.current as any).webkitRequestFullscreen) {
        // Safari
        (videoRef.current as any).webkitRequestFullscreen();
      } else if ((videoRef.current as any).msRequestFullscreen) {
        // IE11
        (videoRef.current as any).msRequestFullscreen();
      }
      setIsFullscreen(true);
    } else {
      if (document.exitFullscreen) {
        document.exitFullscreen();
      } else if ((document as any).webkitExitFullscreen) {
        (document as any).webkitExitFullscreen();
      } else if ((document as any).msExitFullscreen) {
        (document as any).msExitFullscreen();
      }
      setIsFullscreen(false);
    }
  };

  useEffect(() => {
    const video = videoRef.current;
    if (!video) return;

    const updateTime = () => {
      setCurrentTime(video.currentTime);
      setDuration(video.duration);
    };

    const handlePlay = () => setIsPlaying(true);
    const handlePause = () => setIsPlaying(false);
    const handleEnded = () => setIsPlaying(false);
    const handleError = () => setError("Error loading video stream");

    video.addEventListener("timeupdate", updateTime);
    video.addEventListener("play", handlePlay);
    video.addEventListener("pause", handlePause);
    video.addEventListener("ended", handleEnded);
    video.addEventListener("error", handleError);

    return () => {
      video.removeEventListener("timeupdate", updateTime);
      video.removeEventListener("play", handlePlay);
      video.removeEventListener("pause", handlePause);
      video.removeEventListener("ended", handleEnded);
      video.removeEventListener("error", handleError);
    };
  }, []);

  useEffect(() => {
    if (videoRef.current) {
      videoRef.current.volume = volume;
    }
  }, [volume]);

  const formatTime = (time: number) => {
    const minutes = Math.floor(time / 60);
    const seconds = Math.floor(time % 60);
    return `${minutes}:${seconds < 10 ? "0" : ""}${seconds}`;
  };

  return (
    <div className="iptv-player" style={playerStyle}>
      {error && (
        <div className="error-message" style={errorStyle}>
          {error}
        </div>
      )}
      <video ref={videoRef} src={streamUrl} style={videoStyle} />
      <div className="controls" style={controlsStyle}>
        <button onClick={togglePlay} style={buttonStyle}>
          {isPlaying ? "❚❚" : "▶"}
        </button>
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          value={volume}
          onChange={handleVolumeChange}
          style={volumeStyle}
        />
        <button onClick={toggleFullscreen} style={buttonStyle}>
          {isFullscreen ? "⛶" : "⛗"}
        </button>
        <div className="time-display" style={timeStyle}>
          {formatTime(currentTime)} / {formatTime(duration)}
        </div>
      </div>
    </div>
  );
};

// Dark theme styles matching CineVault Premium
const playerStyle: React.CSSProperties = {
  position: "relative",
  width: "100%",
  maxWidth: "800px",
  margin: "0 auto",
  backgroundColor: "#1a1a1a",
  borderRadius: "8px",
  overflow: "hidden",
  boxShadow: "0 4px 6px rgba(0, 0, 0, 0.1)",
};

const videoStyle: React.CSSProperties = {
  width: "100%",
  height: "auto",
  display: "block",
};

const controlsStyle: React.CSSProperties = {
  position: "absolute",
  bottom: 0,
  left: 0,
  right: 0,
  background: "linear-gradient(to top, rgba(0,0,0,0.8), transparent)",
  padding: "12px",
  display: "flex",
  alignItems: "center",
  gap: "12px",
};

const buttonStyle: React.CSSProperties = {
  background: "transparent",
  border: "none",
  color: "#ffffff",
  fontSize: "18px",
  cursor: "pointer",
  padding: "8px",
  borderRadius: "4px",
  transition: "background-color 0.2s",
};

const volumeStyle: React.CSSProperties = {
  flexGrow: 1,
  height: "4px",
  background: "#333",
  borderRadius: "2px",
  outline: "none",
};

const timeStyle: React.CSSProperties = {
  color: "#ffffff",
  fontSize: "14px",
  minWidth: "80px",
  textAlign: "center",
};

const errorStyle: React.CSSProperties = {
  position: "absolute",
  top: "50%",
  left: "50%",
  transform: "translate(-50%, -50%)",
  backgroundColor: "rgba(0,0,0,0.8)",
  color: "#ff4444",
  padding: "16px",
  borderRadius: "8px",
  textAlign: "center",
};

export default IPTVPlayer;
