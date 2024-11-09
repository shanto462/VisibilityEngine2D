using System.Windows.Controls;
using System.Windows.Media;
using VisibilityIn2DGrid.Helper.UI.Constants;

namespace VisibilityIn2DGrid.Helper.UI.Controllers;

public class ZoomController(ScaleTransform zoomTransform, TextBlock zoomLevelText, ViewState viewState)
{
    private readonly ScaleTransform _zoomTransform = zoomTransform;
    private readonly TextBlock _zoomLevelText = zoomLevelText;
    private readonly ViewState _viewState = viewState;

    public void ZoomIn()
    {
        AdjustZoom(ViewConstants.ZoomSpeed);
    }

    public void ZoomOut()
    {
        AdjustZoom(1 / ViewConstants.ZoomSpeed);
    }

    public void ResetZoom()
    {
        _viewState.CurrentZoom = 1.0;
        _zoomTransform.ScaleX = 1.0;
        _zoomTransform.ScaleY = 1.0;
        _zoomLevelText.Text = "100%";
    }

    private void AdjustZoom(double factor)
    {
        double newZoom = _viewState.CurrentZoom * factor;
        newZoom = Math.Max(ViewConstants.ZoomMin, Math.Min(ViewConstants.ZoomMax, newZoom));

        if (Math.Abs(newZoom - _viewState.CurrentZoom) < 0.001)
        {
            return;
        }

        _viewState.CurrentZoom = newZoom;
        _zoomTransform.ScaleX = newZoom;
        _zoomTransform.ScaleY = newZoom;
        _zoomLevelText.Text = $"{newZoom:P0}";
    }
}
