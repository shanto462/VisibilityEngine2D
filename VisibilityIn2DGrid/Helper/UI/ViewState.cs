using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using System.Windows;
using VisibilityIn2DGrid.Enums;

namespace VisibilityIn2DGrid.Helper.UI;

public class ViewState
{
    public double CurrentZoom { get; set; } = 1.0;
    public Point Center { get; set; }
    public bool IsDragging { get; set; }
    public Point? LastMousePosition { get; set; }
    public DateTime LastClickTime { get; set; } = DateTime.MinValue;
    public bool ShowRays { get; set; }
    public ViewMode CurrentViewMode { get; set; } = ViewMode.ShadowCast;
    public float FrustumFOVAngle { get; set; } = 90.0f;
    public float FrustumDirection { get; set; } = 0.0f;
}
