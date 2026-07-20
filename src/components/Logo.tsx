import { ImgHTMLAttributes } from "react";
import iconSource from "../assets/icon-source.png";

export function Logo({ className, ...props }: ImgHTMLAttributes<HTMLImageElement>) {
  return (
    <img 
      src={iconSource} 
      className={className} 
      alt="Vispeak Logo" 
      {...props} 
    />
  );
}
