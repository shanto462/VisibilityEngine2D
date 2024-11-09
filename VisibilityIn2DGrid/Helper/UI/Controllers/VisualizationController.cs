using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Shapes;
using VisibilityIn2DGrid.Culling;
using VisibilityIn2DGrid.Enums;
using VisibilityIn2DGrid.Helper.UI.Constants;
using VisibilityIn2DGrid.RayTracing;

namespace VisibilityIn2DGrid.Helper.UI.Controllers;

public class VisualizationController(Canvas canvas, ViewState viewState, CanvasManager canvasManager)
{
    private readonly Canvas _canvas = canvas;
    private readonly ViewState _viewState = viewState;
    private readonly CanvasManager _canvasManager = canvasManager;

    private Polygon? _visibilityPolygon;
    private Polygon? _frustumPolygon;
    private Polygon? _occlusionPolygon;
    private readonly List<(Polygon, Brush)> _frustumPolygons = [];
    private readonly List<(Polygon, Brush)> _occlusionPolygons = [];
    private readonly OcclusionCuller _occlusionCuller = new();

    public void ExecuteAlgorithm()
    {
        RemoveViews();

        switch (_viewState.CurrentViewMode)
        {
            case ViewMode.FrustumCulling:
                TimingHelper.Time(CalculateFrustum, nameof(CalculateFrustum));
                break;
            case ViewMode.OcclusionCulling:
                TimingHelper.Time(CalculateOcclusion, nameof(CalculateOcclusion));
                break;
            default:
                TimingHelper.Time(CalculateVisibility, nameof(CalculateVisibility));
                break;
        }
    }

    private void CalculateVisibility()
    {
        using ShadowCast2D shadowCast2D = new(_viewState.Center, ViewConstants.VisibilityRange);

        List<Polygon>? obstacles = _canvasManager.GetSpatialIndex()?.QueryBounds(
            _viewState.Center,
            ViewConstants.VisibilityRange
        )?.ToList();

        foreach (Polygon? obstacle in obstacles ?? [])
        {
            shadowCast2D.AddPolygon(obstacle);
        }

        _visibilityPolygon = shadowCast2D.ComputeVisibility();
        _ = _canvas.Children.Add(_visibilityPolygon);
    }

    private void CalculateFrustum()
    {
        FrustumCuller.ViewFrustum frustum = new(
            _viewState.Center,
            _viewState.FrustumFOVAngle,
            _viewState.FrustumDirection,
            ViewConstants.VisibilityRange
        );

        Index.SpatialIndex? spatialIndex = _canvasManager.GetSpatialIndex();
        if (spatialIndex == null)
        {
            return;
        }

        (IList<Polygon>? visiblePolygons, Polygon? fovPolygon) = spatialIndex.QueryFOV(
            _viewState.Center,
            _viewState.FrustumFOVAngle,
            _viewState.FrustumDirection,
            ViewConstants.VisibilityRange
        );

        List<Polygon> filteredPolygons = FrustumCuller.GetVisiblePolygons(visiblePolygons, frustum);

        _frustumPolygons.Clear();
        foreach (Polygon visiblePolygon in filteredPolygons)
        {
            _frustumPolygons.Add((visiblePolygon, visiblePolygon.Fill));
            visiblePolygon.Fill = Brushes.Black;
        }

        fovPolygon.Fill = new SolidColorBrush(Color.FromArgb(90, 255, 255, 0));
        fovPolygon.Stroke = new SolidColorBrush(Colors.Yellow);
        fovPolygon.StrokeThickness = 2;

        _frustumPolygon = fovPolygon;
        _ = _canvas.Children.Add(_frustumPolygon);
    }

    private void CalculateOcclusion()
    {
        List<Polygon> potentialPolygons = _canvasManager.GetSpatialIndex()?.QueryBounds(
            _viewState.Center,
            ViewConstants.VisibilityRange
        )?.ToList() ?? [];

        (List<Polygon> visiblePolygons, Polygon viewArea, List<OcclusionCuller.VisibilityRay> rays) = _occlusionCuller.CalculateVisibility(
            _viewState.Center,
            _viewState.FrustumFOVAngle,
            _viewState.FrustumDirection,
            ViewConstants.VisibilityRange,
            potentialPolygons
        );

        _occlusionPolygons.Clear();
        foreach (Polygon visiblePolygon in visiblePolygons)
        {
            _occlusionPolygons.Add((visiblePolygon, visiblePolygon.Fill));
            visiblePolygon.Fill = Brushes.Black;
        }

        _occlusionPolygon = viewArea;
        _ = _canvas.Children.Add(_occlusionPolygon);

        if (_viewState.ShowRays)
        {
            foreach (OcclusionCuller.VisibilityRay ray in rays)
            {
                Line line = new()
                {
                    X1 = ray.Start.X,
                    Y1 = ray.Start.Y,
                    X2 = ray.End.X,
                    Y2 = ray.End.Y,
                    Stroke = ray.IsBlocked ? Brushes.Red : Brushes.Green,
                    StrokeThickness = 1,
                    Opacity = 0.5
                };
                _canvasManager.AddVisualizationRay(line);
            }
        }
    }

    private void RemoveViews()
    {
        if (_visibilityPolygon != null)
        {
            _canvas.Children.Remove(_visibilityPolygon);
        }

        if (_frustumPolygon != null)
        {
            _canvas.Children.Remove(_frustumPolygon);
        }

        if (_occlusionPolygon != null)
        {
            _canvas.Children.Remove(_occlusionPolygon);
        }

        foreach ((Polygon polygon, Brush brush) in _frustumPolygons)
        {
            polygon.Fill = brush;
        }

        foreach ((Polygon polygon, Brush brush) in _occlusionPolygons)
        {
            polygon.Fill = brush;
        }

        _canvasManager.ClearVisualizationRays();
    }
}
